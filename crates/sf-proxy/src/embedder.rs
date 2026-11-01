//! Embedder seam. Slice 1 ships the mock. TEI adapters land in Slice 4.

use async_trait::async_trait;
use sf_kernels::{AlignedVec1024, DomainHypercubeLock, DIM};

const SPECTRAL_MARKER: &str = "permutation_noise";
const SPECTRAL_IN_BOX_VOTES: usize = 75;
const OUTSIDE_MARGIN: f32 = 1.0e-3;

#[async_trait]
pub trait BatchEmbedderClient: Send + Sync {
    async fn embed_batch_f32(&self, texts: &[&str]) -> Result<Vec<AlignedVec1024>, String>;

    /// Synthetic centroids are not bge-m3 points. Differential locks calibrated on real
    /// embeddings false-breach every mock vector.
    fn is_mock(&self) -> bool {
        false
    }
}

/// Maps the exact token `BREACH` to `breach`, every other text to `pass_vector`.
pub struct MockBatchEmbedder {
    pub pass_vector: AlignedVec1024,
    pub breach_vector: AlignedVec1024,
}

#[async_trait]
impl BatchEmbedderClient for MockBatchEmbedder {
    fn is_mock(&self) -> bool {
        true
    }

    async fn embed_batch_f32(&self, texts: &[&str]) -> Result<Vec<AlignedVec1024>, String> {
        Ok(texts
            .iter()
            .map(|text| {
                if *text == "BREACH" {
                    self.breach_vector
                } else {
                    self.pass_vector
                }
            })
            .collect())
    }
}

/// Case-insensitive fixture token. The playground's Spectral L3 chip carries it.
pub fn contains_permutation_noise(text: &str) -> bool {
    let needle = SPECTRAL_MARKER.as_bytes();
    text.as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

/// Starts at the centroid and steps the nearest bound on every active coordinate past the
/// 75th in-box vote. Inactive coordinates stay on the centroid so cosine does not collapse.
pub fn project_spectral_drift(lock: &DomainHypercubeLock) -> AlignedVec1024 {
    let mut vector = lock.centroid;
    let mut votes = 0usize;
    for dim in 0..DIM {
        if !lock.noise_mask.is_active(dim) {
            continue;
        }
        if votes < SPECTRAL_IN_BOX_VOTES {
            vector.data[dim] = inside_value(
                lock.lo_dilated.data[dim],
                lock.hi_dilated.data[dim],
                lock.centroid.data[dim],
            );
            votes += 1;
        } else {
            vector.data[dim] = nearest_outside(
                lock.centroid.data[dim],
                lock.lo_dilated.data[dim],
                lock.hi_dilated.data[dim],
            );
        }
    }
    vector
}

fn inside_value(lo: f32, hi: f32, preferred: f32) -> f32 {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    if preferred >= lo && preferred <= hi {
        preferred
    } else {
        lo.midpoint(hi)
    }
}

fn nearest_outside(value: f32, lo: f32, hi: f32) -> f32 {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let below = lo - OUTSIDE_MARGIN;
    let above = hi + OUTSIDE_MARGIN;
    if value <= lo {
        return if below.is_finite() {
            below
        } else {
            lo.next_down()
        };
    }
    if value >= hi {
        return if above.is_finite() {
            above
        } else {
            hi.next_up()
        };
    }
    if (value - lo) <= (hi - value) {
        if below.is_finite() {
            below
        } else {
            lo.next_down()
        }
    } else if above.is_finite() {
        above
    } else {
        hi.next_up()
    }
}
