//! Boundary check for embedder outputs. The norm is accumulated in `f64`.

use sf_kernels::{AlignedVec1024, DIM};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizeError {
    BadLen { len: usize },
    NonFinite { index: usize },
    ZeroNorm,
}

impl std::fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadLen { len } => write!(f, "expected {DIM} floats, got {len}"),
            Self::NonFinite { index } => write!(f, "non-finite embedding coordinate {index}"),
            Self::ZeroNorm => write!(f, "embedding norm is zero"),
        }
    }
}

impl std::error::Error for NormalizeError {}

pub fn validate_and_normalize_f32_slice(src: &[f32]) -> Result<AlignedVec1024, NormalizeError> {
    if src.len() != DIM {
        return Err(NormalizeError::BadLen { len: src.len() });
    }
    let mut norm_sq = 0.0f64;
    for (index, value) in src.iter().copied().enumerate() {
        if !value.is_finite() {
            return Err(NormalizeError::NonFinite { index });
        }
        let widened = f64::from(value);
        norm_sq += widened * widened;
    }
    if norm_sq == 0.0 {
        return Err(NormalizeError::ZeroNorm);
    }
    let inv = 1.0 / norm_sq.sqrt();
    let mut out = AlignedVec1024::zeros();
    for (index, value) in src.iter().copied().enumerate() {
        out.data[index] = (f64::from(value) * inv) as f32;
    }
    Ok(out)
}
