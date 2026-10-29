//! Runtime dispatch: AVX-512, then AVX2+FMA, then NEON, then scalar.
//! On this Apple M4 the executed path is NEON. The x86 kernels stay cfg-gated.

use crate::scalar::{scalar_cosine_f64, scalar_hypercube_bitmask};
use crate::{AlignedVec1024, DomainHypercubeLock, NOISE_MASK_WORDS};

#[cfg(target_arch = "x86_64")]
mod avx2;
#[cfg(target_arch = "x86_64")]
mod avx512;
#[cfg(target_arch = "aarch64")]
mod neon;

#[inline]
pub fn dispatch_cosine_f64(q: &AlignedVec1024, c: &AlignedVec1024) -> f64 {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx512f") {
            // SAFETY: avx512f was detected on this core.
            return unsafe { avx512::cosine_f64_avx512(q, c) };
        }
        if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
            // SAFETY: avx2 and fma were detected on this core.
            return unsafe { avx2::cosine_f64_avx2(q, c) };
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: NEON is mandatory on aarch64.
        return unsafe { neon::cosine_f64_neon(q, c) };
    }
    #[allow(unreachable_code)]
    scalar_cosine_f64(q, c)
}

#[inline]
pub fn dispatch_hypercube_bitmask(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
) -> [u64; NOISE_MASK_WORDS] {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx512f") {
            return unsafe { avx512::hypercube_bitmask_avx512(q, lock) };
        }
        if std::is_x86_feature_detected!("avx2") {
            return unsafe { avx2::hypercube_bitmask_avx2(q, lock) };
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        return unsafe { neon::hypercube_bitmask_neon(q, lock) };
    }
    #[allow(unreachable_code)]
    scalar_hypercube_bitmask(q, lock)
}
