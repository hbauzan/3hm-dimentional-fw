//! Envelope, centroid, and SNR noise pruning. Accumulators stay in `f64`.

use sf_kernels::{NoiseBitmask1024, DIM};
use sha2::Digest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationError {
    pub message: String,
}

impl std::fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CalibrationError {}

pub struct Calibration {
    pub lo: [f32; DIM],
    pub hi: [f32; DIM],
    pub centroid: [f32; DIM],
    pub mu: [f64; DIM],
    pub noise_mask: NoiseBitmask1024,
    pub top_k: [u16; 103],
    pub sha256: [u8; 32],
    pub rows: u64,
}

pub fn calibrate_matrix(
    rows: &[f32],
    extra_pruned_dims: usize,
) -> Result<Calibration, CalibrationError> {
    if !rows.len().is_multiple_of(DIM) || rows.is_empty() {
        return Err(CalibrationError {
            message: format!(
                "matrix length {} is not a positive multiple of {DIM}",
                rows.len()
            ),
        });
    }
    let n = rows.len() / DIM;
    let mut lo = [f32::INFINITY; DIM];
    let mut hi = [f32::NEG_INFINITY; DIM];
    let mut sum = [0.0f64; DIM];
    for (dim, value) in rows.iter().copied().enumerate() {
        if !value.is_finite() {
            return Err(CalibrationError {
                message: format!("non-finite sample at flat index {dim}"),
            });
        }
        let coord = dim % DIM;
        lo[coord] = lo[coord].min(value);
        hi[coord] = hi[coord].max(value);
        sum[coord] += f64::from(value);
    }
    let inv_n = 1.0 / n as f64;
    let mut mu = [0.0f64; DIM];
    for dim in 0..DIM {
        mu[dim] = sum[dim] * inv_n;
    }
    let mut norm_sq = 0.0f64;
    for value in mu {
        norm_sq += value * value;
    }
    let inv_norm = if norm_sq > 0.0 {
        1.0 / norm_sq.sqrt()
    } else {
        0.0
    };
    let mut centroid = [0.0f32; DIM];
    for dim in 0..DIM {
        centroid[dim] = (mu[dim] * inv_norm) as f32;
    }
    let noise_mask = calibrate_noise_bitmask(&mu, &lo, &hi, extra_pruned_dims);
    let top_k = top_k_indices(&mu, &lo, &hi, &noise_mask);
    let mut bytes = Vec::with_capacity(rows.len() * 4);
    for value in rows {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let digest = sha2::Sha256::digest(&bytes);
    let mut sha256 = [0u8; 32];
    sha256.copy_from_slice(&digest);
    Ok(Calibration {
        lo,
        hi,
        centroid,
        mu,
        noise_mask,
        top_k,
        sha256,
        rows: n as u64,
    })
}

pub fn calibrate_noise_bitmask(
    mu: &[f64; DIM],
    lo: &[f32; DIM],
    hi: &[f32; DIM],
    extra_pruned_dims: usize,
) -> NoiseBitmask1024 {
    let mut mask = NoiseBitmask1024::new_bge_m3_default();
    if extra_pruned_dims == 0 {
        return mask;
    }
    let mut ranked: Vec<(f64, usize)> = (0..DIM)
        .filter(|dim| mask.is_active(*dim))
        .map(|dim| (snr(mu[dim], lo[dim], hi[dim]), dim))
        .collect();
    ranked.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    for (_, dim) in ranked.into_iter().take(extra_pruned_dims) {
        mask.prune_coordinate(dim as u16);
    }
    mask
}

pub fn top_k_indices(
    mu: &[f64; DIM],
    lo: &[f32; DIM],
    hi: &[f32; DIM],
    mask: &NoiseBitmask1024,
) -> [u16; 103] {
    let mut ranked: Vec<(f64, usize)> = (0..DIM)
        .filter(|dim| mask.is_active(*dim))
        .map(|dim| (snr(mu[dim], lo[dim], hi[dim]), dim))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    let mut top_k = [292u16; 103];
    for (slot, (_, dim)) in top_k.iter_mut().zip(ranked.into_iter().take(103)) {
        *slot = dim as u16;
    }
    top_k
}

fn snr(mu: f64, lo: f32, hi: f32) -> f64 {
    let width = f64::from(hi) - f64::from(lo);
    mu.abs() / (width + f64::from(f32::EPSILON))
}

pub fn read_f32_matrix(bytes: &[u8]) -> Result<Vec<f32>, CalibrationError> {
    if bytes.len() < 8 {
        return Err(CalibrationError {
            message: "f32 matrix file is missing the row-count header".to_string(),
        });
    }
    let mut header = [0u8; 8];
    header.copy_from_slice(&bytes[..8]);
    let rows = u64::from_le_bytes(header) as usize;
    let payload = &bytes[8..];
    let expected = rows.saturating_mul(DIM).saturating_mul(4);
    if payload.len() != expected {
        return Err(CalibrationError {
            message: format!(
                "payload {} bytes, expected {expected} for {rows} rows",
                payload.len()
            ),
        });
    }
    let mut out = Vec::with_capacity(rows * DIM);
    for chunk in payload.chunks_exact(4) {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(chunk);
        out.push(f32::from_le_bytes(buf));
    }
    Ok(out)
}
