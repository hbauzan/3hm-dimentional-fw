use sf_kernels::{AlignedVec1024, DomainHypercubeLock, LockSpec, NoiseBitmask1024, MAX_TOPK};
use sf_storage::HotRegistry;

fn lock(domain_id: &str) -> DomainHypercubeLock {
    DomainHypercubeLock::from_spec(LockSpec {
        domain_id: domain_id.to_string(),
        centroid: AlignedVec1024::zeros(),
        lo: AlignedVec1024::zeros(),
        hi: AlignedVec1024::zeros(),
        epsilon: 0.0,
        noise_mask: NoiseBitmask1024::new_bge_m3_default(),
        top_k_indices: [0u16; MAX_TOPK],
        top_k_len: 1,
        quorum_k: 103,
        l1_cosine_threshold: 0.5,
        l2_clause_threshold: 0.5,
        polarity: 0,
        domain_version: 1,
    })
}

#[test]
fn hot_registry_returns_the_ten_pairs_that_touch_a_canonical_domain() {
    let registry = HotRegistry::from_locks([
        lock("python"),
        lock("receta"),
        lock("python_receta"),
        lock("legal_python"),
        lock("arquitectura_python"),
        lock("astronomia_python"),
        lock("botanica_python"),
        lock("filosofia_python"),
        lock("finanzas_python"),
        lock("geologia_python"),
        lock("medicina_python"),
        lock("musica_python"),
        lock("legal_receta"),
    ]);
    let python = registry.pairwise_locks_for("python");
    let ids: Vec<&str> = python.iter().map(|item| item.domain_id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "arquitectura_python",
            "astronomia_python",
            "botanica_python",
            "filosofia_python",
            "finanzas_python",
            "geologia_python",
            "legal_python",
            "medicina_python",
            "musica_python",
            "python_receta",
        ]
    );
    assert!(registry.pairwise_locks_for("python_receta").is_empty());
    let receta_locks = registry.pairwise_locks_for("receta");
    let receta: Vec<&str> = receta_locks
        .iter()
        .map(|item| item.domain_id.as_str())
        .collect();
    assert_eq!(receta, vec!["legal_receta", "python_receta"]);
}
