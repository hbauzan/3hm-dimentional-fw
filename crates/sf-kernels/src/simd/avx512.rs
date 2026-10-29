//! AVX-512F kernels. Not executed on aarch64 hosts.

use crate::{AlignedVec1024, DomainHypercubeLock, DIM, NOISE_MASK_WORDS};
use std::arch::x86_64::*;

#[target_feature(enable = "avx512f")]
#[inline]
pub unsafe fn cosine_f64_avx512(q: &AlignedVec1024, c: &AlignedVec1024) -> f64 {
    let mut dot0 = _mm512_setzero_pd();
    let mut dot1 = _mm512_setzero_pd();
    let mut nq0 = _mm512_setzero_pd();
    let mut nq1 = _mm512_setzero_pd();
    let mut nc0 = _mm512_setzero_pd();
    let mut nc1 = _mm512_setzero_pd();
    let q_ptr = q.data.as_ptr();
    let c_ptr = c.data.as_ptr();
    let mut i = 0;
    while i < DIM {
        let q_ps = _mm512_load_ps(q_ptr.add(i));
        let c_ps = _mm512_load_ps(c_ptr.add(i));
        let q_lo = _mm512_cvtps_pd(_mm512_castps512_ps256(q_ps));
        let c_lo = _mm512_cvtps_pd(_mm512_castps512_ps256(c_ps));
        let q_hi = _mm512_cvtps_pd(_mm512_extractf32x8_ps(q_ps, 1));
        let c_hi = _mm512_cvtps_pd(_mm512_extractf32x8_ps(c_ps, 1));
        dot0 = _mm512_fmadd_pd(q_lo, c_lo, dot0);
        dot1 = _mm512_fmadd_pd(q_hi, c_hi, dot1);
        nq0 = _mm512_fmadd_pd(q_lo, q_lo, nq0);
        nq1 = _mm512_fmadd_pd(q_hi, q_hi, nq1);
        nc0 = _mm512_fmadd_pd(c_lo, c_lo, nc0);
        nc1 = _mm512_fmadd_pd(c_hi, c_hi, nc1);
        i += 16;
    }
    let dot = _mm512_reduce_add_pd(_mm512_add_pd(dot0, dot1));
    let nq = _mm512_reduce_add_pd(_mm512_add_pd(nq0, nq1));
    let nc = _mm512_reduce_add_pd(_mm512_add_pd(nc0, nc1));
    let norm_sq = nq * nc;
    if norm_sq <= 0.0 {
        0.0
    } else {
        dot / norm_sq.sqrt()
    }
}

#[target_feature(enable = "avx512f")]
#[inline]
pub unsafe fn hypercube_bitmask_avx512(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
) -> [u64; NOISE_MASK_WORDS] {
    let mut out = [0u64; NOISE_MASK_WORDS];
    let q_ptr = q.data.as_ptr();
    let lo_ptr = lock.lo_dilated.data.as_ptr();
    let hi_ptr = lock.hi_dilated.data.as_ptr();
    for (word_idx, slot) in out.iter_mut().enumerate() {
        let base = word_idx << 6;
        let mut word = 0u64;
        for chunk in 0..4 {
            let offset = base + (chunk << 4);
            let qv = _mm512_load_ps(q_ptr.add(offset));
            let lov = _mm512_load_ps(lo_ptr.add(offset));
            let hiv = _mm512_load_ps(hi_ptr.add(offset));
            let ge = _mm512_cmp_ps_mask(qv, lov, _CMP_GE_OQ);
            let le = _mm512_cmp_ps_mask(qv, hiv, _CMP_LE_OQ);
            let bits = u64::from(ge & le);
            word |= bits << (chunk << 4);
        }
        *slot = word & lock.noise_mask.words[word_idx];
    }
    out
}
