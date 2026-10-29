//! Scalar numerical kernels for the semantic firewall.
//! SIMD dispatch replaces the cosine and bitmask entry points in Slice 2.
#![forbid(unsafe_code)]

mod scalar;
mod types;

pub use scalar::{
    evaluate_l3_gate, scalar_cosine_f64, scalar_cosine_similarity, scalar_hypercube_bitmask,
    COSINE_PARITY_ABS_BOUND,
};
pub use types::*;

/// Cosine used by the proxy. Slice 1 is the scalar reference.
#[inline]
pub fn cosine_similarity_1024(q: &AlignedVec1024, c: &AlignedVec1024) -> f32 {
    scalar_cosine_similarity(q, c)
}

/// Hypercube membership mask. Slice 1 is the scalar reference.
#[inline]
pub fn evaluate_hypercube_bitmask(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
) -> [u64; NOISE_MASK_WORDS] {
    scalar_hypercube_bitmask(q, lock)
}
