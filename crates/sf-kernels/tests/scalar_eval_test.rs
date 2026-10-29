use sf_kernels::{
    evaluate_l3_gate, AlignedVec1024, DomainHypercubeLock, L3InspectionMode, LockSpec,
    NoiseBitmask1024, DIM, MAX_TOPK,
};

fn lock_around(lo: f32, hi: f32, epsilon: f32) -> DomainHypercubeLock {
    let mut top_k_indices = [0u16; MAX_TOPK];
    for (idx, slot) in top_k_indices.iter_mut().enumerate().take(8) {
        *slot = u16::try_from(idx).unwrap();
    }
    DomainHypercubeLock::from_spec(LockSpec {
        domain_id: "python".to_string(),
        centroid: AlignedVec1024::zeros(),
        lo: AlignedVec1024 { data: [lo; DIM] },
        hi: AlignedVec1024 { data: [hi; DIM] },
        epsilon,
        noise_mask: NoiseBitmask1024::new_bge_m3_default(),
        top_k_indices,
        top_k_len: 8,
        quorum_k: 103,
        l1_cosine_threshold: 0.5,
        l2_clause_threshold: 0.5,
        polarity: 0,
        domain_version: 1,
    })
}

#[test]
fn layout_and_default_mask() {
    assert_eq!(std::mem::size_of::<AlignedVec1024>(), 4096);
    assert_eq!(std::mem::align_of::<AlignedVec1024>(), 64);
    let mask = NoiseBitmask1024::new_bge_m3_default();
    assert_eq!(mask.active_count(), 1015);
    assert!(!mask.is_active(292));
    assert!(!mask.is_active(616));
    assert!(mask.is_active(0));
    let aligned = AlignedVec1024::zeros();
    assert_eq!((std::ptr::from_ref(&aligned) as usize) % 64, 0);
}

#[test]
fn mode_c_passes_inside_box_and_reports_breaches() {
    let lock = lock_around(-1.0, 1.0, 0.0);
    let inside = AlignedVec1024::zeros();
    let pass = evaluate_l3_gate(&inside, &lock, L3InspectionMode::ModeCSpectralQuorum, 4);
    assert!(pass.passed);
    assert_eq!(pass.affirmative_votes, 1015);
    assert!(pass.failing_coordinates.is_empty());

    let outside = AlignedVec1024 { data: [2.0; DIM] };
    let fail = evaluate_l3_gate(&outside, &lock, L3InspectionMode::ModeCSpectralQuorum, 4);
    assert!(!fail.passed);
    assert_eq!(fail.affirmative_votes, 0);
    assert_eq!(fail.required_votes, 103);
    assert_eq!(fail.failing_coordinates.len(), 4);
    assert_eq!(fail.failing_coordinates[0].dim, 0);
    assert_eq!(fail.failing_coordinates[0].observed_bits, 2.0f32.to_bits());
}

#[test]
fn pre_dilated_bounds_include_eps_margin() {
    let lock = lock_around(0.0, 0.0, 0.25);
    assert_eq!(lock.lo_dilated.data[3].to_bits(), (-0.25f32).to_bits());
    assert_eq!(lock.hi_dilated.data[3].to_bits(), 0.25f32.to_bits());

    let mut query = AlignedVec1024::zeros();
    query.data.fill(0.2);
    let pass = evaluate_l3_gate(&query, &lock, L3InspectionMode::ModeBStrictHypercube, 2);
    assert!(pass.passed);

    query.data.fill(0.3);
    let fail = evaluate_l3_gate(&query, &lock, L3InspectionMode::ModeBStrictHypercube, 2);
    assert!(!fail.passed);
    assert!(!fail.failing_coordinates.is_empty());
}
