//! Full 1024-dimensional diagnostic inspector and comparative DTOs.
//!
//! Evaluates coordinate-level classifications (PRUNED_UNIVERSAL, PRUNED_SNR,
//! ACTIVE_PASS, ACTIVE_BREACH) across the entire 1024D hypercube, and provides
//! simultaneous 3-mode comparative evaluation (Mode A, Mode B, Mode C).

use serde::{Deserialize, Serialize};
use sf_kernels::{
    evaluate_l3_gate, scalar_cosine_similarity, AlignedVec1024, DomainHypercubeLock,
    L3InspectionMode, DIM, UNIVERSAL_NOISE_INDICES,
};

/// Coordinate classification telemetry cell for the 32x32 diagnostic heatmap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoordinateCellDto {
    pub dim: u16,
    pub status: String,
    pub is_top_k: bool,
    pub observed_f32: f32,
    pub observed_hex: String,
    pub lo_bound_f32: f32,
    pub hi_bound_f32: f32,
    pub delta_f32: f32,
}

/// Evaluation statistics for a single L3 strategy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModeStatsDto {
    pub passed: bool,
    pub affirmative_votes: u16,
    pub required_votes: u16,
    pub evaluated_dimensions: u16,
}

/// Side-by-side comparison across Scalar Cosine and L3 Modes A, B, and C.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModeComparisonDto {
    pub scalar_cosine: f32,
    pub scalar_cosine_hex: String,
    pub cosine_threshold: f32,
    pub cosine_passed: bool,
    pub mode_a: ModeStatsDto,
    pub mode_b: ModeStatsDto,
    pub mode_c: ModeStatsDto,
}

/// Check if dimension is one of the 9 universal BGE-M3 background noise coordinates.
#[inline]
pub fn is_universal_noise(dim: u16) -> bool {
    UNIVERSAL_NOISE_INDICES.contains(&dim)
}

/// Inspect all 1,024 coordinates of an evaluated vector against a domain lock.
pub fn inspect_1024_coordinates(
    q: &AlignedVec1024,
    lock: &DomainHypercubeLock,
) -> Vec<CoordinateCellDto> {
    let top_k_slice = &lock.top_k_indices[..lock.top_k_len as usize];
    let mut cells = Vec::with_capacity(DIM);

    for dim in 0..DIM {
        let dim_u16 = dim as u16;
        let val = q.data[dim];
        let lo = lock.lo_dilated.data[dim];
        let hi = lock.hi_dilated.data[dim];
        let is_top_k = top_k_slice.contains(&dim_u16);
        let is_universal = is_universal_noise(dim_u16);
        let is_active = lock.noise_mask.is_active(dim);

        let (status, delta) = if is_universal {
            let delta = if val < lo {
                lo - val
            } else if val > hi {
                val - hi
            } else {
                0.0
            };
            ("PRUNED_UNIVERSAL", delta)
        } else if !is_active {
            let delta = if val < lo {
                lo - val
            } else if val > hi {
                val - hi
            } else {
                0.0
            };
            ("PRUNED_SNR", delta)
        } else if val >= lo && val <= hi {
            ("ACTIVE_PASS", 0.0)
        } else {
            let delta = if val < lo { lo - val } else { val - hi };
            ("ACTIVE_BREACH", delta)
        };

        cells.push(CoordinateCellDto {
            dim: dim_u16,
            status: status.to_string(),
            is_top_k,
            observed_f32: val,
            observed_hex: format!("0x{:08x}", val.to_bits()),
            lo_bound_f32: lo,
            hi_bound_f32: hi,
            delta_f32: delta,
        });
    }

    cells
}

/// Compute simultaneous 3-mode L3 comparison and scalar cosine baseline.
pub fn compare_modes(q: &AlignedVec1024, lock: &DomainHypercubeLock) -> ModeComparisonDto {
    let cos = scalar_cosine_similarity(q, &lock.centroid);
    let cosine_passed = if lock.polarity == 0 {
        cos >= lock.l1_cosine_threshold
    } else {
        cos < lock.l1_cosine_threshold
    };

    let eval_a = evaluate_l3_gate(q, lock, L3InspectionMode::ModeATopK, 0);
    let eval_b = evaluate_l3_gate(q, lock, L3InspectionMode::ModeBStrictHypercube, 0);
    let eval_c = evaluate_l3_gate(q, lock, L3InspectionMode::ModeCSpectralQuorum, 0);

    ModeComparisonDto {
        scalar_cosine: cos,
        scalar_cosine_hex: format!("0x{:08x}", cos.to_bits()),
        cosine_threshold: lock.l1_cosine_threshold,
        cosine_passed,
        mode_a: ModeStatsDto {
            passed: eval_a.passed,
            affirmative_votes: eval_a.affirmative_votes,
            required_votes: eval_a.required_votes,
            evaluated_dimensions: eval_a.evaluated_dimensions,
        },
        mode_b: ModeStatsDto {
            passed: eval_b.passed,
            affirmative_votes: eval_b.affirmative_votes,
            required_votes: eval_b.required_votes,
            evaluated_dimensions: eval_b.evaluated_dimensions,
        },
        mode_c: ModeStatsDto {
            passed: eval_c.passed,
            affirmative_votes: eval_c.affirmative_votes,
            required_votes: eval_c.required_votes,
            evaluated_dimensions: eval_c.evaluated_dimensions,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_bench::generate_canonical_locks;

    #[test]
    fn test_coordinate_classification_counts() {
        let canonical_locks = generate_canonical_locks();
        let lock = &canonical_locks[0];

        assert_eq!(lock.noise_mask.active_count(), 985);
        let q = AlignedVec1024::zeros();
        let cells = inspect_1024_coordinates(&q, lock);

        assert_eq!(cells.len(), 1024);

        let mut universal_count = 0;
        let mut snr_count = 0;
        let mut active_pass = 0;
        let mut active_breach = 0;

        for cell in &cells {
            match cell.status.as_str() {
                "PRUNED_UNIVERSAL" => universal_count += 1,
                "PRUNED_SNR" => snr_count += 1,
                "ACTIVE_PASS" => active_pass += 1,
                "ACTIVE_BREACH" => active_breach += 1,
                other => panic!("unexpected status: {other}"),
            }
        }

        assert_eq!(
            universal_count, 9,
            "must have exactly 9 universal noise dims"
        );
        assert_eq!(snr_count, 30, "must have exactly 30 SNR noise dims");
        assert_eq!(active_pass + active_breach, 985, "active count must be 985");
        assert_eq!(
            universal_count + snr_count + active_pass + active_breach,
            1024
        );

        // Verify exact universal noise indices
        for &idx in &UNIVERSAL_NOISE_INDICES {
            assert_eq!(
                cells[idx as usize].status, "PRUNED_UNIVERSAL",
                "dim {idx} must be PRUNED_UNIVERSAL"
            );
        }
    }

    #[test]
    fn test_compare_modes_simultaneous() {
        let canonical_locks = generate_canonical_locks();
        let lock = &canonical_locks[0];
        let q = AlignedVec1024::zeros();

        let comparison = compare_modes(&q, lock);
        assert_eq!(comparison.mode_a.evaluated_dimensions, 103);
        assert_eq!(comparison.mode_b.evaluated_dimensions, 985);
        assert_eq!(comparison.mode_c.evaluated_dimensions, 985);
        assert_eq!(comparison.mode_c.required_votes, 103);
    }
}
