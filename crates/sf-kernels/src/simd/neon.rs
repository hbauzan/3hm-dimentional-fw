//! NEON kernels. Cosine uses the same even/odd `f64` FMA chain as the scalar reference
//! so the absolute error stays under `1e-14` on arbitrary finite inputs.

use crate::{AlignedVec1024, DomainHypercubeLock, DIM, NOISE_MASK_WORDS};
use std::arch::aarch64::*;

#[target_feature(enable = "neon")]
#[inline]
pub unsafe fn cosine_f64_neon(q: &AlignedVec1024, c: &AlignedVec1024) -> f64 {
    let mut dot_a = vdupq_n_f64(0.0);
    let mut dot_b = vdupq_n_f64(0.0);
    let mut dot_c = vdupq_n_f64(0.0);
    let mut dot_d = vdupq_n_f64(0.0);
    let mut nq_a = vdupq_n_f64(0.0);
    let mut nq_b = vdupq_n_f64(0.0);
    let mut nq_c = vdupq_n_f64(0.0);
    let mut nq_d = vdupq_n_f64(0.0);
    let mut nc_a = vdupq_n_f64(0.0);
    let mut nc_b = vdupq_n_f64(0.0);
    let mut nc_c = vdupq_n_f64(0.0);
    let mut nc_d = vdupq_n_f64(0.0);
    let q_ptr = q.data.as_ptr();
    let c_ptr = c.data.as_ptr();
    let mut i = 0;
    while i < DIM {
        let q0 = vcvt_f64_f32(vld1_f32(q_ptr.add(i)));
        let c0 = vcvt_f64_f32(vld1_f32(c_ptr.add(i)));
        let q1 = vcvt_f64_f32(vld1_f32(q_ptr.add(i + 2)));
        let c1 = vcvt_f64_f32(vld1_f32(c_ptr.add(i + 2)));
        let q2 = vcvt_f64_f32(vld1_f32(q_ptr.add(i + 4)));
        let c2 = vcvt_f64_f32(vld1_f32(c_ptr.add(i + 4)));
        let q3 = vcvt_f64_f32(vld1_f32(q_ptr.add(i + 6)));
        let c3 = vcvt_f64_f32(vld1_f32(c_ptr.add(i + 6)));
        dot_a = vfmaq_f64(dot_a, q0, c0);
        dot_b = vfmaq_f64(dot_b, q1, c1);
        dot_c = vfmaq_f64(dot_c, q2, c2);
        dot_d = vfmaq_f64(dot_d, q3, c3);
        nq_a = vfmaq_f64(nq_a, q0, q0);
        nq_b = vfmaq_f64(nq_b, q1, q1);
        nq_c = vfmaq_f64(nq_c, q2, q2);
        nq_d = vfmaq_f64(nq_d, q3, q3);
        nc_a = vfmaq_f64(nc_a, c0, c0);
        nc_b = vfmaq_f64(nc_b, c1, c1);
        nc_c = vfmaq_f64(nc_c, c2, c2);
        nc_d = vfmaq_f64(nc_d, c3, c3);
        i += 8;
    }
    let dot = vaddvq_f64(vaddq_f64(vaddq_f64(dot_a, dot_b), vaddq_f64(dot_c, dot_d)));
    let nq = vaddvq_f64(vaddq_f64(vaddq_f64(nq_a, nq_b), vaddq_f64(nq_c, nq_d)));
    let nc = vaddvq_f64(vaddq_f64(vaddq_f64(nc_a, nc_b), vaddq_f64(nc_c, nc_d)));
    let norm_sq = nq * nc;
    if norm_sq <= 0.0 {
        0.0
    } else {
        dot / norm_sq.sqrt()
    }
}

#[target_feature(enable = "neon")]
#[inline]
pub unsafe fn hypercube_bitmask_neon(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
) -> [u64; NOISE_MASK_WORDS] {
    let mut out = [0u64; NOISE_MASK_WORDS];
    let q_ptr = q.data.as_ptr();
    let lo_ptr = lock.lo_dilated.data.as_ptr();
    let hi_ptr = lock.hi_dilated.data.as_ptr();
    let weights = vld1q_u32([1u32, 2, 4, 8].as_ptr());

    for (word_idx, slot) in out.iter_mut().enumerate() {
        let base = word_idx << 6;
        let mut word_mask = 0u64;
        for chunk in 0..16 {
            let offset = base + (chunk << 2);
            let qv = vld1q_f32(q_ptr.add(offset));
            let lov = vld1q_f32(lo_ptr.add(offset));
            let hiv = vld1q_f32(hi_ptr.add(offset));
            let ge = vcgeq_f32(qv, lov);
            let le = vcleq_f32(qv, hiv);
            let both = vandq_u32(ge, le);
            let bits01 = vshrq_n_u32::<31>(both);
            let nibble = vaddvq_u32(vmulq_u32(bits01, weights)) as u64;
            word_mask |= nibble << (chunk << 2);
        }
        *slot = word_mask & lock.noise_mask.words[word_idx];
    }
    out
}
