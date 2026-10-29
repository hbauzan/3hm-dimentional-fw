//! Cold-path copy from an Arrow buffer into an aligned tensor.

use sf_kernels::{AlignedVec1024, DIM};
use std::sync::atomic::{AtomicU64, Ordering};

/// Incremented when a source slice is not 64-byte aligned. The copy still happens.
pub static UNALIGNED_ARROW_BUFFER_LOADS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    BadLen { len: usize },
    NonFinite { dim: usize },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadLen { len } => write!(f, "expected {DIM} floats, got {len}"),
            Self::NonFinite { dim } => write!(f, "non-finite coordinate at dim {dim}"),
        }
    }
}

impl std::error::Error for LoadError {}

pub fn load_validated_vec1024(src: &[f32]) -> Result<AlignedVec1024, LoadError> {
    if src.len() != DIM {
        return Err(LoadError::BadLen { len: src.len() });
    }
    for (dim, value) in src.iter().copied().enumerate() {
        if !value.is_finite() {
            return Err(LoadError::NonFinite { dim });
        }
    }
    if !(src.as_ptr() as usize).is_multiple_of(64) {
        UNALIGNED_ARROW_BUFFER_LOADS.fetch_add(1, Ordering::Relaxed);
    }
    let mut out = AlignedVec1024::zeros();
    out.data.copy_from_slice(src);
    Ok(out)
}
