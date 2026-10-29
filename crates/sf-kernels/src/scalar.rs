//! Scalar reference kernels. Dot products and norms accumulate in `f64`.

use crate::types::{
    AlignedVec1024, CoordinateBreach, DomainHypercubeLock, L3EvalResult, L3InspectionMode, DIM,
    NOISE_MASK_WORDS,
};

/// Absolute bound for SIMD-vs-scalar cosine on the `f64` result (architect Q3).
pub const COSINE_PARITY_ABS_BOUND: f64 = 1e-14;

#[inline]
pub fn scalar_cosine_f64(q: &AlignedVec1024, c: &AlignedVec1024) -> f64 {
    let mut dot0 = 0.0f64;
    let mut dot1 = 0.0f64;
    let mut nq0 = 0.0f64;
    let mut nq1 = 0.0f64;
    let mut nc0 = 0.0f64;
    let mut nc1 = 0.0f64;

    let mut i = 0;
    while i < DIM {
        let q0 = f64::from(q.data[i]);
        let c0 = f64::from(c.data[i]);
        let q1 = f64::from(q.data[i + 1]);
        let c1 = f64::from(c.data[i + 1]);
        dot0 = q0.mul_add(c0, dot0);
        dot1 = q1.mul_add(c1, dot1);
        nq0 = q0.mul_add(q0, nq0);
        nq1 = q1.mul_add(q1, nq1);
        nc0 = c0.mul_add(c0, nc0);
        nc1 = c1.mul_add(c1, nc1);
        i += 2;
    }

    let dot = dot0 + dot1;
    let norm_sq = (nq0 + nq1) * (nc0 + nc1);
    if norm_sq <= 0.0 {
        0.0
    } else {
        dot / norm_sq.sqrt()
    }
}

#[inline]
pub fn scalar_cosine_similarity(q: &AlignedVec1024, c: &AlignedVec1024) -> f32 {
    scalar_cosine_f64(q, c) as f32
}

#[inline]
fn coord_inside(val: f32, lo: f32, hi: f32) -> bool {
    val >= lo && val <= hi
}

#[inline]
pub fn scalar_hypercube_bitmask(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
) -> [u64; NOISE_MASK_WORDS] {
    let mut out = [0u64; NOISE_MASK_WORDS];
    for (word_idx, slot) in out.iter_mut().enumerate() {
        let noise_word = lock.noise_mask.words[word_idx];
        let base_dim = word_idx << 6;
        let mut pass_word = 0u64;
        for bit in 0..64 {
            let dim = base_dim + bit;
            let val = q.data[dim];
            let inside = coord_inside(val, lock.lo_dilated.data[dim], lock.hi_dilated.data[dim]);
            pass_word |= u64::from(inside) << bit;
        }
        *slot = pass_word & noise_word;
    }
    out
}

fn polarity_passes(polarity: u8, have: u16, required: u16) -> bool {
    if polarity == 0 {
        have >= required
    } else {
        have < required
    }
}

fn push_breach(
    out: &mut Vec<CoordinateBreach>,
    dim: usize,
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
    limit: usize,
) -> bool {
    if out.len() >= limit {
        return true;
    }
    let val = q.data[dim];
    let lo = lock.lo_dilated.data[dim];
    let hi = lock.hi_dilated.data[dim];
    let delta = if val < lo { lo - val } else { val - hi };
    out.push(CoordinateBreach {
        dim: dim as u16,
        observed_val: val,
        observed_bits: val.to_bits(),
        lo_bound: lo,
        hi_bound: hi,
        delta_out_of_bounds: delta,
    });
    out.len() >= limit
}

/// Tier-3 evaluation on the scalar bitmask. Allocates breach telemetry only on failure.
pub fn evaluate_l3_gate(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
    mode: L3InspectionMode,
    max_telemetry_breaches: usize,
) -> L3EvalResult {
    match mode {
        L3InspectionMode::ModeATopK => eval_mode_a(q, lock, max_telemetry_breaches),
        L3InspectionMode::ModeBStrictHypercube | L3InspectionMode::ModeCSpectralQuorum => {
            eval_mode_bc(q, lock, mode, max_telemetry_breaches)
        }
    }
}

fn eval_mode_a(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
    max_telemetry_breaches: usize,
) -> L3EvalResult {
    let k_len = lock.top_k_len as usize;
    let mut votes: u16 = 0;
    let mut evaluated: u16 = 0;
    for idx in 0..k_len {
        let dim = lock.top_k_indices[idx] as usize;
        if !lock.noise_mask.is_active(dim) {
            continue;
        }
        evaluated += 1;
        let val = q.data[dim];
        if coord_inside(val, lock.lo_dilated.data[dim], lock.hi_dilated.data[dim]) {
            votes += 1;
        }
    }
    let required = evaluated;
    let passed = polarity_passes(lock.polarity, votes, required);
    let mut failing_coordinates = Vec::new();
    if !passed && max_telemetry_breaches > 0 {
        for idx in 0..k_len {
            let dim = lock.top_k_indices[idx] as usize;
            if !lock.noise_mask.is_active(dim) {
                continue;
            }
            let val = q.data[dim];
            if !coord_inside(val, lock.lo_dilated.data[dim], lock.hi_dilated.data[dim])
                && push_breach(
                    &mut failing_coordinates,
                    dim,
                    q,
                    lock,
                    max_telemetry_breaches,
                )
            {
                break;
            }
        }
    }
    L3EvalResult {
        passed,
        affirmative_votes: votes,
        evaluated_dimensions: evaluated,
        required_votes: required,
        failing_coordinates,
    }
}

fn eval_mode_bc(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
    mode: L3InspectionMode,
    max_telemetry_breaches: usize,
) -> L3EvalResult {
    let pass_mask = crate::evaluate_hypercube_bitmask(q, lock);
    let votes: u16 = pass_mask.iter().map(|word| word.count_ones() as u16).sum();
    let active_dims = u16::try_from(lock.noise_mask.active_count()).unwrap_or(u16::MAX);
    let required = match mode {
        L3InspectionMode::ModeBStrictHypercube => active_dims,
        L3InspectionMode::ModeCSpectralQuorum => lock.quorum_k,
        L3InspectionMode::ModeATopK => unreachable!("mode A is handled above"),
    };
    let passed = polarity_passes(lock.polarity, votes, required);
    let mut failing_coordinates = Vec::new();
    if !passed && max_telemetry_breaches > 0 {
        'outer: for (word_idx, pass_word) in pass_mask.iter().enumerate() {
            let mut failed_bits = lock.noise_mask.words[word_idx] & !pass_word;
            while failed_bits != 0 {
                let bit = failed_bits.trailing_zeros() as usize;
                let dim = (word_idx << 6) + bit;
                if push_breach(
                    &mut failing_coordinates,
                    dim,
                    q,
                    lock,
                    max_telemetry_breaches,
                ) {
                    break 'outer;
                }
                failed_bits &= failed_bits - 1;
            }
        }
    }
    L3EvalResult {
        passed,
        affirmative_votes: votes,
        evaluated_dimensions: active_dims,
        required_votes: required,
        failing_coordinates,
    }
}
