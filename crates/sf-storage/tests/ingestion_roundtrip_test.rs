use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sf_storage::{
    calibrate_matrix, calibrate_noise_bitmask, load_registry, write_calibration, IngestSpec, DIM,
};

#[test]
fn extra_pruned_dims_reaches_985_active() {
    let mu = [0.01f64; DIM];
    let lo = [0.0f32; DIM];
    let hi = [1.0f32; DIM];
    let mask = calibrate_noise_bitmask(&mu, &lo, &hi, 30);
    assert_eq!(mask.active_count(), 985);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn five_hundred_vectors_roundtrip_lo_bits() {
    let mut rng = ChaCha8Rng::seed_from_u64(0x1A4C_E500);
    let mut rows = vec![0.0f32; 500 * DIM];
    for value in &mut rows {
        *value = rng.gen::<f32>() * 0.02 - 0.01;
    }
    let calibration = calibrate_matrix(&rows, 0).unwrap();
    let dir = std::env::temp_dir().join(format!("sf-lance-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let uri = dir.to_str().unwrap();
    write_calibration(IngestSpec {
        uri,
        domain_id: "python",
        version: 1,
        polarity: 0,
        l1: 0.4,
        l2: 0.35,
        epsilon: 0.0,
        quorum_k: 103,
        calibration: &calibration,
    })
    .await
    .unwrap();
    let registry = load_registry(uri).await.unwrap();
    let lock = registry.get("python").unwrap();
    for dim in 0..DIM {
        assert_eq!(
            lock.lo_dilated.data[dim].to_bits(),
            calibration.lo[dim].to_bits(),
            "lo dim {dim}"
        );
        assert_eq!(
            lock.hi_dilated.data[dim].to_bits(),
            calibration.hi[dim].to_bits(),
            "hi dim {dim}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
