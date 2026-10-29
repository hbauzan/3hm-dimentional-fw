use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sf_kernels::{
    cosine_similarity_1024_f64, evaluate_hypercube_bitmask, scalar_cosine_f64,
    scalar_hypercube_bitmask, AlignedVec1024, DomainHypercubeLock, LockSpec, NoiseBitmask1024,
    COSINE_PARITY_ABS_BOUND, DIM, MAX_TOPK,
};

fn finite_bits(rng: &mut ChaCha8Rng) -> u32 {
    let mut bits = rng.gen::<u32>();
    if ((bits >> 23) & 0xff) == 0xff {
        bits &= 0x807f_ffff;
    }
    bits
}

fn random_finite(rng: &mut ChaCha8Rng) -> AlignedVec1024 {
    let mut vector = AlignedVec1024::zeros();
    for lane in &mut vector.data {
        *lane = f32::from_bits(finite_bits(rng));
    }
    vector
}

fn random_mask_vector(rng: &mut ChaCha8Rng) -> AlignedVec1024 {
    let mut vector = AlignedVec1024::zeros();
    for lane in &mut vector.data {
        *lane = f32::from_bits(rng.gen::<u32>());
    }
    vector
}

fn lock_with_bounds(lo: f32, hi: f32) -> DomainHypercubeLock {
    DomainHypercubeLock::from_spec(LockSpec {
        domain_id: "fuzz".to_string(),
        centroid: AlignedVec1024::zeros(),
        lo: AlignedVec1024 { data: [lo; DIM] },
        hi: AlignedVec1024 { data: [hi; DIM] },
        epsilon: 0.0,
        noise_mask: NoiseBitmask1024::new_bge_m3_default(),
        top_k_indices: [0u16; MAX_TOPK],
        top_k_len: 0,
        quorum_k: 103,
        l1_cosine_threshold: 0.0,
        l2_clause_threshold: 0.0,
        polarity: 0,
        domain_version: 1,
    })
}

#[test]
fn simd_matches_scalar_on_ten_thousand_vectors() {
    let mut rng = ChaCha8Rng::seed_from_u64(0x5F1C_0512);
    let lock = lock_with_bounds(-0.25, 0.5);

    let on_boundary = AlignedVec1024 { data: [-0.25; DIM] };
    assert_eq!(
        evaluate_hypercube_bitmask(&on_boundary, &lock),
        scalar_hypercube_bitmask(&on_boundary, &lock)
    );

    for index in 0..10_000 {
        let left = random_finite(&mut rng);
        let right = if index % 64 == 0 {
            let mut sub = AlignedVec1024::zeros();
            for lane in &mut sub.data {
                *lane = f32::from_bits(rng.gen::<u32>() & 0x007f_ffff);
            }
            sub
        } else {
            random_finite(&mut rng)
        };
        let simd = cosine_similarity_1024_f64(&left, &right);
        let scalar = scalar_cosine_f64(&left, &right);
        let delta = (simd - scalar).abs();
        assert!(
            delta < COSINE_PARITY_ABS_BOUND,
            "cosine delta {delta} at vector {index}: simd={simd} scalar={scalar}"
        );

        let query = random_mask_vector(&mut rng);
        assert_eq!(
            evaluate_hypercube_bitmask(&query, &lock),
            scalar_hypercube_bitmask(&query, &lock),
            "mask mismatch at vector {index}"
        );
    }
}
