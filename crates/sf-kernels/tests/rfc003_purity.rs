//! RFC-003: a 16-ULP micro-gap survives binary32 and collapses under round-4 and binary16.

use half::f16;
use sf_kernels::{
    evaluate_l3_gate, AlignedVec1024, DomainHypercubeLock, L3InspectionMode, LockSpec,
    NoiseBitmask1024, DIM, MAX_TOPK, UNIVERSAL_NOISE_INDICES,
};

fn round4(value: f32) -> f32 {
    let scaled = value * 10_000.0;
    scaled.round() / 10_000.0
}

fn add_positive_ulps(value: f32, ulps: u32) -> f32 {
    f32::from_bits(value.to_bits() + ulps)
}

fn micro_gap_lock() -> (DomainHypercubeLock, f32, f32) {
    let edge = 0.03125f32;
    let adversarial = add_positive_ulps(edge, 16);
    let lock = DomainHypercubeLock::from_spec(LockSpec {
        domain_id: "rfc003".to_string(),
        centroid: AlignedVec1024::zeros(),
        lo: AlignedVec1024 { data: [edge; DIM] },
        hi: AlignedVec1024 { data: [edge; DIM] },
        epsilon: 0.0,
        noise_mask: NoiseBitmask1024::new_bge_m3_default(),
        top_k_indices: [0u16; MAX_TOPK],
        top_k_len: 0,
        quorum_k: 103,
        l1_cosine_threshold: 0.0,
        l2_clause_threshold: 0.0,
        polarity: 0,
        domain_version: 1,
    });
    (lock, edge, adversarial)
}

#[test]
fn pure_f32_rejects_micro_gap_that_round4_and_f16_accept() {
    let (lock, edge, adversarial) = micro_gap_lock();
    assert!(adversarial > edge);
    assert!(adversarial - edge > 0.0);

    let mut active = Vec::new();
    for dim in 0..DIM {
        if lock.noise_mask.is_active(dim) {
            active.push(dim);
        }
    }
    assert!(active.len() >= 920);
    assert_eq!(active.len(), 1015);
    for index in UNIVERSAL_NOISE_INDICES {
        assert!(!active.contains(&(index as usize)));
    }

    let query = AlignedVec1024 {
        data: [adversarial; DIM],
    };
    let result = evaluate_l3_gate(&query, &lock, L3InspectionMode::ModeCSpectralQuorum, 4);
    assert!(!result.passed);
    assert_eq!(result.affirmative_votes, 0);
    assert!(!result.failing_coordinates.is_empty());

    let mut round4_inside = 0usize;
    let mut f16_inside = 0usize;
    for dim in active.iter().take(920) {
        let observed = query.data[*dim];
        let lo = lock.lo_dilated.data[*dim];
        let hi = lock.hi_dilated.data[*dim];
        assert!(observed > hi);
        let rounded = round4(observed);
        if rounded >= round4(lo) && rounded <= round4(hi) {
            round4_inside += 1;
        }
        let narrow = f16::from_f32(observed).to_f32();
        let lo16 = f16::from_f32(lo).to_f32();
        let hi16 = f16::from_f32(hi).to_f32();
        if narrow >= lo16 && narrow <= hi16 {
            f16_inside += 1;
        }
    }
    assert_eq!(round4_inside, 920);
    assert_eq!(f16_inside, 920);
}
