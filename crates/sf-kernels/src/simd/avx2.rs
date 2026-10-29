//! AVX2+FMA kernels. Not executed on aarch64 hosts.

use crate::{AlignedVec1024, DomainHypercubeLock, DIM, NOISE_MASK_WORDS};
use std::arch::x86_64::*;

#[target_feature(enable = "avx2,fma")]
#[inline]
pub unsafe fn cosine_f64_avx2(q: &AlignedVec1024, c: &AlignedVec1024) -> f64 {
    let mut dot = _mm256_setzero_pd();
    let mut nq = _mm256_setzero_pd();
    let mut nc = _mm256_setzero_pd();
    let q_ptr = q.data.as_ptr();
    let c_ptr = c.data.as_ptr();
    let mut i = 0;
    while i < DIM {
        let qv = _mm256_cvtps_pd(_mm_loadu_ps(q_ptr.add(i)));
        let cv = _mm256_cvtps_pd(_mm_loadu_ps(c_ptr.add(i)));
        dot = _mm256_fmadd_pd(qv, cv, dot);
        nq = _mm256_fmadd_pd(qv, qv, nq);
        nc = _mm256_fmadd_pd(cv, cv, nc);
        i += 4;
    }
    let dot_s = hsum256(dot);
    let nq_s = hsum256(nq);
    let nc_s = hsum256(nc);
    let norm_sq = nq_s * nc_s;
    if norm_sq <= 0.0 {
        0.0
    } else {
        dot_s / norm_sq.sqrt()
    }
}

#[target_feature(enable = "avx2,fma")]
#[inline]
unsafe fn hsum256(v: __m256d) -> f64 {
    let hi = _mm256_extractf128_pd(v, 1);
    let lo = _mm256_castpd256_pd128(v);
    let sum = _mm_add_pd(lo, hi);
    let shuf = _mm_unpackhi_pd(sum, sum);
    let both = _mm_add_sd(sum, shuf);
    _mm_cvtsd_f64(both)
}

#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn hypercube_bitmask_avx2(
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
        for chunk in 0..8 {
            let offset = base + (chunk << 3);
            let qv = _mm256_load_ps(q_ptr.add(offset));
            let lov = _mm256_load_ps(lo_ptr.add(offset));
            let hiv = _mm256_load_ps(hi_ptr.add(offset));
            let ge = _mm256_cmp_ps(qv, lov, _CMP_GE_OQ);
            let le = _mm256_cmp_ps(qv, hiv, _CMP_LE_OQ);
            let both = _mm256_and_ps(ge, le);
            let bits = _mm256_movemask_ps(both) as u64;
            word |= bits << (chunk << 3);
        }
        *slot = word & lock.noise_mask.words[word_idx];
    }
    out
}
