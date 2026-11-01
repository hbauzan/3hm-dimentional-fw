//! Multi-domain routing. A named domain evaluates that lock.
//! `X-Semantic-Domain: *` defaults to AnyAllowlist. `X-SF-Wildcard-Mode: all` requires every lock.

use crate::cascade::{
    evaluate_pairwise_containment, evaluate_three_tier_cascade, pairwise_breach_envelope,
    PairwiseBreachEnvelope,
};
use crate::telemetry::BreachErrorEnvelope;
use axum::http::HeaderMap;
use sf_kernels::{cosine_similarity_1024, AlignedVec1024, DomainHypercubeLock, L3InspectionMode};
use sf_storage::HotRegistry;
use std::sync::Arc;

#[derive(Debug)]
pub struct RouteError {
    pub status_code: &'static str,
    pub message: String,
}

struct LockSets {
    denylists: Vec<Arc<DomainHypercubeLock>>,
    allowlists: Vec<Arc<DomainHypercubeLock>>,
    require_all_allowlists: bool,
    named_domain: bool,
}

pub enum RouteFailure {
    Breach(Box<BreachErrorEnvelope>),
    Pairwise(Box<PairwiseBreachEnvelope>),
    Route(RouteError),
}

pub struct RouteInput<'a> {
    pub registry: &'a HotRegistry,
    pub headers: &'a HeaderMap,
    pub default_domain: &'a str,
    pub full_prompt: &'a AlignedVec1024,
    pub clause_vecs: &'a [AlignedVec1024],
    pub clauses: &'a [&'a str],
    pub l3_mode: L3InspectionMode,
    pub max_telemetry_coords: usize,
}

pub fn evaluate_route(input: RouteInput<'_>) -> Result<u64, Box<RouteFailure>> {
    let RouteInput {
        registry,
        headers,
        default_domain,
        full_prompt,
        clause_vecs,
        clauses,
        l3_mode,
        max_telemetry_coords,
    } = input;
    let sets = match lock_sets(registry, headers, default_domain) {
        Ok(sets) => sets,
        Err(err) => return Err(Box::new(RouteFailure::Route(err))),
    };
    let mut latency = 0u64;
    for lock in &sets.denylists {
        latency = evaluate_three_tier_cascade(
            full_prompt,
            clause_vecs,
            clauses,
            lock,
            l3_mode,
            max_telemetry_coords,
        )
        .map_err(|err| Box::new(RouteFailure::Breach(err)))?;
        latency = contain_named_domain(&sets, registry, lock, full_prompt, clause_vecs, latency)?;
    }
    if sets.allowlists.is_empty() {
        return Ok(latency);
    }
    if sets.require_all_allowlists {
        for lock in &sets.allowlists {
            latency = evaluate_three_tier_cascade(
                full_prompt,
                clause_vecs,
                clauses,
                lock,
                l3_mode,
                max_telemetry_coords,
            )
            .map_err(|err| Box::new(RouteFailure::Breach(err)))?;
            latency =
                contain_named_domain(&sets, registry, lock, full_prompt, clause_vecs, latency)?;
        }
        return Ok(latency);
    }
    let mut nearest: Option<(f32, Box<BreachErrorEnvelope>)> = None;
    for lock in &sets.allowlists {
        match evaluate_three_tier_cascade(
            full_prompt,
            clause_vecs,
            clauses,
            lock,
            l3_mode,
            max_telemetry_coords,
        ) {
            Ok(ns) => {
                return contain_named_domain(
                    &sets,
                    registry,
                    lock,
                    full_prompt,
                    clause_vecs,
                    ns.max(latency),
                );
            }
            Err(err) => {
                let score = cosine_similarity_1024(full_prompt, &lock.centroid);
                let replace = nearest
                    .as_ref()
                    .map(|(best, _)| score > *best)
                    .unwrap_or(true);
                if replace {
                    nearest = Some((score, err));
                }
            }
        }
    }
    Err(Box::new(RouteFailure::Breach(
        nearest.expect("allowlist set is non-empty").1,
    )))
}

fn lock_sets(
    registry: &HotRegistry,
    headers: &HeaderMap,
    default_domain: &str,
) -> Result<LockSets, RouteError> {
    let requested = headers
        .get("x-semantic-domain")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(default_domain);
    let snapshot = registry.load();
    if requested == "*" {
        let require_all_allowlists = headers
            .get("x-sf-wildcard-mode")
            .and_then(|value| value.to_str().ok())
            == Some("all");
        let mut denylists = Vec::new();
        let mut allowlists = Vec::new();
        for lock in snapshot.locks() {
            if lock.polarity == 1 {
                denylists.push(Arc::clone(lock));
            } else {
                allowlists.push(Arc::clone(lock));
            }
        }
        if allowlists.is_empty() && denylists.is_empty() {
            return Err(RouteError {
                status_code: "UNKNOWN_DOMAIN_LOCK",
                message: "Fail-closed: registry has no locks for wildcard routing.".to_string(),
            });
        }
        return Ok(LockSets {
            denylists,
            allowlists,
            require_all_allowlists,
            named_domain: false,
        });
    }
    let Some(lock) = snapshot.get(requested) else {
        return Err(RouteError {
            status_code: "UNKNOWN_DOMAIN_LOCK",
            message: format!("Fail-closed: domain '{requested}' not found in registry."),
        });
    };
    if lock.polarity == 1 {
        Ok(LockSets {
            denylists: vec![lock],
            allowlists: Vec::new(),
            require_all_allowlists: false,
            named_domain: true,
        })
    } else {
        Ok(LockSets {
            denylists: Vec::new(),
            allowlists: vec![lock],
            require_all_allowlists: false,
            named_domain: true,
        })
    }
}

fn contain_named_domain(
    sets: &LockSets,
    registry: &HotRegistry,
    lock: &DomainHypercubeLock,
    full_prompt: &AlignedVec1024,
    clause_vecs: &[AlignedVec1024],
    latency: u64,
) -> Result<u64, Box<RouteFailure>> {
    if !sets.named_domain {
        return Ok(latency);
    }
    let pairs = registry.pairwise_locks_for(&lock.domain_id);
    if pairs.is_empty() {
        return Ok(latency);
    }
    evaluate_pairwise_containment(full_prompt, clause_vecs, &lock.domain_id, &pairs)
        .map(|()| latency)
        .map_err(|breach| {
            Box::new(RouteFailure::Pairwise(Box::new(pairwise_breach_envelope(
                breach,
            ))))
        })
}
