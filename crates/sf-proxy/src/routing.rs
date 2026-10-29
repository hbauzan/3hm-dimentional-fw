//! Multi-domain routing. A named domain evaluates that lock.
//! `X-Semantic-Domain: *` defaults to AnyAllowlist. `X-SF-Wildcard-Mode: all` requires every lock.

use crate::cascade::evaluate_three_tier_cascade;
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
}

pub enum RouteFailure {
    Breach(Box<BreachErrorEnvelope>),
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
            Ok(ns) => return Ok(ns.max(latency)),
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
        })
    } else {
        Ok(LockSets {
            denylists: Vec::new(),
            allowlists: vec![lock],
            require_all_allowlists: false,
        })
    }
}
