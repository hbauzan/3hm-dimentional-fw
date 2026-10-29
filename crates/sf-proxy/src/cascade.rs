//! L1 → L2 → L3 cascade. The first failing tier returns the breach envelope.

use crate::telemetry::{
    f32_hex, BreachErrorBody, BreachErrorEnvelope, BreachMetricsDto, BreachTelemetryDto,
    CoordinateBreachDto,
};
use sf_kernels::{
    cosine_similarity_1024, evaluate_l3_gate, AlignedVec1024, DomainHypercubeLock, L3InspectionMode,
};
use sha2::{Digest, Sha256};
use std::time::Instant;

pub fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn polarity_label(polarity: u8) -> &'static str {
    if polarity == 0 {
        "POSITIVE_ALLOWLIST"
    } else {
        "NEGATIVE_DENYLIST"
    }
}

fn mode_label(mode: L3InspectionMode) -> &'static str {
    match mode {
        L3InspectionMode::ModeATopK => "MODE_A_TOPK",
        L3InspectionMode::ModeBStrictHypercube => "MODE_B_STRICT_HYPERCUBE",
        L3InspectionMode::ModeCSpectralQuorum => "MODE_C_SPECTRAL_QUORUM",
    }
}

fn tier_passes(polarity: u8, score: f32, threshold: f32) -> bool {
    if polarity == 0 {
        score >= threshold
    } else {
        score < threshold
    }
}

struct GateClock {
    start: Instant,
}

impl GateClock {
    fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    fn elapsed_ns(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

pub fn evaluate_three_tier_cascade(
    full_prompt_vec: &AlignedVec1024,
    clause_vecs: &[AlignedVec1024],
    clauses_text: &[&str],
    lock: &DomainHypercubeLock,
    l3_mode: L3InspectionMode,
    max_telemetry_coords: usize,
) -> Result<u64, Box<BreachErrorEnvelope>> {
    let clock = GateClock::start();
    let polarity = polarity_label(lock.polarity);
    let mode = mode_label(l3_mode);
    let l1_cos = cosine_similarity_1024(full_prompt_vec, &lock.centroid);

    if !tier_passes(lock.polarity, l1_cos, lock.l1_cosine_threshold) {
        return Err(Box::new(breach(BreachParts {
            lock,
            polarity,
            mode,
            failing_tier: "L1_CATEGORICAL_COSINE",
            breaching_clause_index: None,
            breaching_clause_sha256: None,
            total_clauses_extracted: clause_vecs.len(),
            gate_latency_ns: clock.elapsed_ns(),
            metrics: metrics_l1(l1_cos, lock),
            message: format!(
                "Fail-closed containment breach at Tier L1_CATEGORICAL_COSINE for domain '{}'.",
                lock.domain_id
            ),
        })));
    }

    let mut last_l2 = l1_cos;
    for (idx, clause_vec) in clause_vecs.iter().enumerate() {
        let clause_cos = cosine_similarity_1024(clause_vec, &lock.centroid);
        last_l2 = clause_cos;
        if !tier_passes(lock.polarity, clause_cos, lock.l2_clause_threshold) {
            let clause_hash = sha256_hex(clauses_text.get(idx).copied().unwrap_or(""));
            return Err(Box::new(breach(BreachParts {
                lock,
                polarity,
                mode,
                failing_tier: "L2_SLIDING_WINDOW",
                breaching_clause_index: Some(idx),
                breaching_clause_sha256: Some(clause_hash),
                total_clauses_extracted: clause_vecs.len(),
                gate_latency_ns: clock.elapsed_ns(),
                metrics: metrics_l2(l1_cos, clause_cos, lock),
                message: format!(
                    "Fail-closed containment breach at Tier L2_SLIDING_WINDOW on clause {idx} for domain '{}'.",
                    lock.domain_id
                ),
            })));
        }
    }

    for (idx, clause_vec) in clause_vecs.iter().enumerate() {
        let l3 = evaluate_l3_gate(clause_vec, lock, l3_mode, max_telemetry_coords);
        if !l3.passed {
            let clause_hash = sha256_hex(clauses_text.get(idx).copied().unwrap_or(""));
            let failing_coordinates = l3
                .failing_coordinates
                .into_iter()
                .map(|item| CoordinateBreachDto {
                    dim: item.dim,
                    observed_val: item.observed_val,
                    observed_bits: format!("0x{:08x}", item.observed_bits),
                    lo_bound: item.lo_bound,
                    hi_bound: item.hi_bound,
                    delta_out_of_bounds: item.delta_out_of_bounds,
                })
                .collect();
            return Err(Box::new(breach(BreachParts {
                lock,
                polarity,
                mode,
                failing_tier: "L3_DDI_INSPECTOR",
                breaching_clause_index: Some(idx),
                breaching_clause_sha256: Some(clause_hash),
                total_clauses_extracted: clause_vecs.len(),
                gate_latency_ns: clock.elapsed_ns(),
                metrics: BreachMetricsDto {
                    l1_prompt_cosine: l1_cos,
                    l1_prompt_cosine_bits: f32_hex(l1_cos),
                    l1_threshold: lock.l1_cosine_threshold,
                    l2_clause_cosine: Some(last_l2),
                    l2_clause_cosine_bits: Some(f32_hex(last_l2)),
                    l2_threshold: lock.l2_clause_threshold,
                    l3_affirmative_votes: Some(l3.affirmative_votes),
                    l3_required_quorum: Some(l3.required_votes),
                    l3_evaluated_dimensions: Some(l3.evaluated_dimensions),
                    failing_coordinates,
                },
                message: format!(
                    "Fail-closed containment breach at Tier L3_DDI_INSPECTOR ({mode}) on clause {idx} for domain '{}'.",
                    lock.domain_id
                ),
            })));
        }
    }

    Ok(clock.elapsed_ns())
}

fn metrics_l1(l1_cos: f32, lock: &DomainHypercubeLock) -> BreachMetricsDto {
    BreachMetricsDto {
        l1_prompt_cosine: l1_cos,
        l1_prompt_cosine_bits: f32_hex(l1_cos),
        l1_threshold: lock.l1_cosine_threshold,
        l2_clause_cosine: None,
        l2_clause_cosine_bits: None,
        l2_threshold: lock.l2_clause_threshold,
        l3_affirmative_votes: None,
        l3_required_quorum: None,
        l3_evaluated_dimensions: None,
        failing_coordinates: Vec::new(),
    }
}

fn metrics_l2(l1_cos: f32, l2_cos: f32, lock: &DomainHypercubeLock) -> BreachMetricsDto {
    BreachMetricsDto {
        l1_prompt_cosine: l1_cos,
        l1_prompt_cosine_bits: f32_hex(l1_cos),
        l1_threshold: lock.l1_cosine_threshold,
        l2_clause_cosine: Some(l2_cos),
        l2_clause_cosine_bits: Some(f32_hex(l2_cos)),
        l2_threshold: lock.l2_clause_threshold,
        l3_affirmative_votes: None,
        l3_required_quorum: None,
        l3_evaluated_dimensions: None,
        failing_coordinates: Vec::new(),
    }
}

struct BreachParts<'a> {
    lock: &'a DomainHypercubeLock,
    polarity: &'static str,
    mode: &'static str,
    failing_tier: &'static str,
    breaching_clause_index: Option<usize>,
    breaching_clause_sha256: Option<String>,
    total_clauses_extracted: usize,
    gate_latency_ns: u64,
    metrics: BreachMetricsDto,
    message: String,
}

fn breach(parts: BreachParts<'_>) -> BreachErrorEnvelope {
    let BreachParts {
        lock,
        polarity,
        mode,
        failing_tier,
        breaching_clause_index,
        breaching_clause_sha256,
        total_clauses_extracted,
        gate_latency_ns,
        metrics,
        message,
    } = parts;
    BreachErrorEnvelope {
        error: BreachErrorBody {
            kind: "ddi_ingress_breach",
            code: "SEMANTIC_CONTAINMENT_BREACH",
            message,
            telemetry: BreachTelemetryDto {
                domain_id: lock.domain_id.clone(),
                domain_version: lock.domain_version,
                polarity,
                failing_tier,
                l3_mode: mode,
                breaching_clause_index,
                breaching_clause_sha256,
                total_clauses_extracted,
                gate_latency_ns,
                metrics,
            },
        },
    }
}
