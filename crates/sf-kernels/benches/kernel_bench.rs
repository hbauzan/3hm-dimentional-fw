use std::hint::black_box;
use std::time::Instant;

use criterion::Criterion;
use sf_kernels::{
    cosine_similarity_1024, evaluate_l3_gate, AlignedVec1024, DomainHypercubeLock,
    L3InspectionMode, LockSpec, NoiseBitmask1024, DIM, MAX_TOPK,
};

fn sample_lock() -> DomainHypercubeLock {
    let mut centroid = AlignedVec1024::zeros();
    centroid.data[0] = 1.0;
    DomainHypercubeLock::from_spec(LockSpec {
        domain_id: "bench".to_string(),
        centroid,
        lo: AlignedVec1024 { data: [-0.1; DIM] },
        hi: AlignedVec1024 { data: [0.1; DIM] },
        epsilon: 0.0,
        noise_mask: NoiseBitmask1024::new_bge_m3_default(),
        top_k_indices: [0u16; MAX_TOPK],
        top_k_len: 8,
        quorum_k: 103,
        l1_cosine_threshold: 0.2,
        l2_clause_threshold: 0.2,
        polarity: 0,
        domain_version: 1,
    })
}

fn nanos_per_call(iters: u32, mut body: impl FnMut()) -> f64 {
    for _ in 0..4_000 {
        body();
    }
    let mut best = f64::MAX;
    for _ in 0..8 {
        let start = Instant::now();
        for _ in 0..iters {
            body();
        }
        let sample = start.elapsed().as_nanos() as f64 / f64::from(iters);
        if sample < best {
            best = sample;
        }
    }
    best
}

fn assert_budgets() {
    let lock = sample_lock();
    let query = AlignedVec1024 { data: [0.01; DIM] };
    let centroid = lock.centroid;
    let cosine_ns = nanos_per_call(20_000, || {
        black_box(cosine_similarity_1024(
            black_box(&query),
            black_box(&centroid),
        ));
    });
    let l3_ns = nanos_per_call(20_000, || {
        black_box(evaluate_l3_gate(
            black_box(&query),
            black_box(&lock),
            L3InspectionMode::ModeCSpectralQuorum,
            0,
        ));
    });
    println!("cosine_ns={cosine_ns:.1} l3_ns={l3_ns:.1}");
    #[cfg(not(debug_assertions))]
    {
        if cosine_ns > 250.0 {
            eprintln!("cosine {cosine_ns:.1} ns exceeds 250 ns");
            std::process::exit(1);
        }
        if l3_ns > 300.0 {
            eprintln!("l3 {l3_ns:.1} ns exceeds 300 ns");
            std::process::exit(1);
        }
    }
}

fn benches(c: &mut Criterion) {
    let lock = sample_lock();
    let query = AlignedVec1024 { data: [0.01; DIM] };
    let centroid = lock.centroid;
    c.bench_function("cosine_similarity_1024", |b| {
        b.iter(|| cosine_similarity_1024(black_box(&query), black_box(&centroid)))
    });
    c.bench_function("evaluate_l3_gate_mode_c", |b| {
        b.iter(|| {
            evaluate_l3_gate(
                black_box(&query),
                black_box(&lock),
                L3InspectionMode::ModeCSpectralQuorum,
                0,
            )
        })
    });
}

fn main() {
    assert_budgets();
    let mut c = Criterion::default().configure_from_args();
    benches(&mut c);
    c.final_summary();
}
