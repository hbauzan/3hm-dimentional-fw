use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sf_kernels::{AlignedVec1024, DomainHypercubeLock, LockSpec, NoiseBitmask1024, DIM, MAX_TOPK};
use sf_proxy::{router, MockBatchEmbedder, MockUpstreamForwarder, ProxyAppState};
use sf_storage::MemoryRegistry;
use tower::ServiceExt;

fn vector_at(index: usize) -> AlignedVec1024 {
    let mut vector = AlignedVec1024::zeros();
    vector.data[index] = 1.0;
    vector
}

fn lock(id: &str, axis: usize, polarity: u8, contains_unit_axis: bool) -> DomainHypercubeLock {
    let centroid = vector_at(axis);
    let mut lo = AlignedVec1024::zeros();
    let mut hi = AlignedVec1024::zeros();
    if contains_unit_axis {
        lo.data.fill(-0.05);
        hi.data.fill(0.05);
        hi.data[axis] = 1.2;
    } else {
        lo.data.fill(0.5);
        hi.data.fill(0.6);
    }
    DomainHypercubeLock::from_spec(LockSpec {
        domain_id: id.to_string(),
        centroid,
        lo,
        hi,
        epsilon: 0.0,
        noise_mask: NoiseBitmask1024::new_bge_m3_default(),
        top_k_indices: [0u16; MAX_TOPK],
        top_k_len: 4,
        quorum_k: 103,
        l1_cosine_threshold: 0.8,
        l2_clause_threshold: 0.8,
        polarity,
        domain_version: 1,
    })
}

struct Harness {
    app: axum::Router,
}

impl Harness {
    fn new(pass_axis: usize) -> Self {
        let pass = vector_at(pass_axis);
        let state = Arc::new(ProxyAppState {
            registry: MemoryRegistry::from_locks([
                lock("python", 0, 0, true),
                lock("legal", 1, 0, false),
                lock("deny-sql", 2, 1, false),
            ]),
            embedder: Arc::new(MockBatchEmbedder {
                pass_vector: pass,
                breach_vector: AlignedVec1024::zeros(),
            }),
            upstream: Arc::new(MockUpstreamForwarder),
            default_domain: "python".to_string(),
            max_telemetry_coords: 4,
        });
        Self { app: router(state) }
    }

    async fn post(
        &self,
        domain: &str,
        wildcard_mode: Option<&str>,
        body: &str,
    ) -> (StatusCode, String) {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .header("x-semantic-domain", domain);
        if let Some(mode) = wildcard_mode {
            builder = builder.header("x-sf-wildcard-mode", mode);
        }
        let response = self
            .app
            .clone()
            .oneshot(builder.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }
}

#[tokio::test]
async fn wildcard_any_allowlist_passes_when_one_lock_matches() {
    let harness = Harness::new(0);
    let (status, body) = harness
        .post(
            "*",
            None,
            r#"{"messages":[{"role":"user","content":"hello kernel"}]}"#,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("data:"), "{body}");
}

#[tokio::test]
async fn wildcard_all_rejects_when_another_allowlist_misses() {
    let harness = Harness::new(0);
    let (status, body) = harness
        .post(
            "*",
            Some("all"),
            r#"{"messages":[{"role":"user","content":"hello kernel"}]}"#,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(body.contains("SEMANTIC_CONTAINMENT_BREACH"), "{body}");
}

#[tokio::test]
async fn named_denylist_uses_inverted_cosine() {
    let harness = Harness::new(0);
    let (status, body) = harness
        .post(
            "deny-sql",
            None,
            r#"{"messages":[{"role":"user","content":"hello kernel"}]}"#,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[test]
fn dim_constant() {
    assert_eq!(DIM, 1024);
}
