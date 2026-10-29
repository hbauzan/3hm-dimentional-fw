use sf_kernels::DIM;
use sf_proxy::{embed_request, validate_and_normalize_f32_slice};

#[test]
fn grpc_request_forces_normalize_flag() {
    let request = embed_request("clause");
    assert_eq!(request.normalize, Some(true));
    assert_eq!(request.inputs, "clause");
}

#[test]
fn unnormalized_vector_is_forced_to_unit_norm_and_nan_is_rejected() {
    let mut raw = vec![0.0f32; DIM];
    raw[0] = 3.0;
    raw[1] = 4.0;
    let normalized = validate_and_normalize_f32_slice(&raw).unwrap();
    let mut norm_sq = 0.0f64;
    for value in normalized.data {
        norm_sq += f64::from(value) * f64::from(value);
    }
    assert!((norm_sq - 1.0).abs() < 1e-6, "norm_sq={norm_sq}");
    assert!((normalized.data[0] - 0.6).abs() < 1e-5);
    assert!((normalized.data[1] - 0.8).abs() < 1e-5);

    raw[2] = f32::NAN;
    assert!(validate_and_normalize_f32_slice(&raw).is_err());
}
