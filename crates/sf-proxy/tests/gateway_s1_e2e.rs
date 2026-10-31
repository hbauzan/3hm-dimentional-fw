//! Daemon seam: CORS, dual breach JSON, ephemeral chat overrides, concurrent SSE.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::response::Response;
use bytes::Bytes;
use futures_util::stream;
use http_body_util::BodyExt;
use sf_kernels::{AlignedVec1024, DomainHypercubeLock, LockSpec, NoiseBitmask1024, MAX_TOPK};
use sf_proxy::{
    router, router_with_cors, CorsAllow, MockBatchEmbedder, MockUpstreamForwarder, ProxyAppState,
    UpstreamLlmForwarder,
};
use sf_storage::HotRegistry;
use tower::ServiceExt;

fn unit_axis() -> AlignedVec1024 {
    let mut vector = AlignedVec1024::zeros();
    vector.data[0] = 1.0;
    vector
}

fn python_lock(l1_threshold: f32) -> DomainHypercubeLock {
    let centroid = unit_axis();
    let mut lo = AlignedVec1024::zeros();
    let mut hi = AlignedVec1024::zeros();
    lo.data.fill(-0.1);
    hi.data.fill(1.1);
    let mut top_k = [0u16; MAX_TOPK];
    for (idx, slot) in top_k.iter_mut().enumerate().take(4) {
        *slot = u16::try_from(idx).unwrap();
    }
    DomainHypercubeLock::from_spec(LockSpec {
        domain_id: "python".to_string(),
        centroid,
        lo,
        hi,
        epsilon: 0.0,
        noise_mask: NoiseBitmask1024::new_bge_m3_default(),
        top_k_indices: top_k,
        top_k_len: 4,
        quorum_k: 103,
        l1_cosine_threshold: l1_threshold,
        l2_clause_threshold: 0.5,
        polarity: 0,
        domain_version: 1,
    })
}

fn state(
    lock: DomainHypercubeLock,
    upstream: Arc<dyn UpstreamLlmForwarder>,
) -> (Arc<HotRegistry>, Arc<ProxyAppState>) {
    let centroid = lock.centroid;
    let registry = Arc::new(HotRegistry::from_locks([lock]));
    let app_state = Arc::new(ProxyAppState {
        registry: Arc::clone(&registry),
        embedder: Arc::new(MockBatchEmbedder {
            pass_vector: centroid,
            breach_vector: AlignedVec1024::zeros(),
        }),
        upstream,
        default_domain: "python".to_string(),
        max_telemetry_coords: 4,
    });
    (registry, app_state)
}

fn app(lock: DomainHypercubeLock) -> axum::Router {
    let (_registry, app_state) = state(lock, Arc::new(MockUpstreamForwarder));
    router(app_state)
}

async fn oneshot(app: axum::Router, request: Request<Body>) -> (StatusCode, HeaderMap, String) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn preflight_uses_configured_origin_on_both_routes() {
    let allow = CorsAllow::parse(Some("https://studio.example")).expect("origin");
    let (_registry, app_state) = state(python_lock(0.5), Arc::new(MockUpstreamForwarder));
    let app = router_with_cors(app_state, allow);
    for path in ["/v1/chat/completions", "/api/v1/chat"] {
        let (status, headers, body) = oneshot(
            app.clone(),
            Request::builder()
                .method("OPTIONS")
                .uri(path)
                .header("origin", "https://studio.example")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{path} {body}");
        assert_eq!(
            headers
                .get("access-control-allow-origin")
                .and_then(|value| value.to_str().ok()),
            Some("https://studio.example"),
            "{path}"
        );
        let methods = headers
            .get("access-control-allow-methods")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        assert!(methods.contains("POST"), "{path} {methods}");
    }
}

#[tokio::test]
async fn unset_origin_allows_any() {
    let allow = CorsAllow::parse(None).expect("fallback");
    assert!(matches!(allow, CorsAllow::Any));
    let (_registry, app_state) = state(python_lock(0.5), Arc::new(MockUpstreamForwarder));
    let (status, headers, _) = oneshot(
        router_with_cors(app_state, allow),
        Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/chat")
            .header("origin", "https://other.example")
            .header("access-control-request-method", "POST")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );
}

#[test]
fn cors_parse_rejects_a_header_illegal_origin() {
    let parsed = CorsAllow::parse(Some("https://studio.example\r\nX-Injected: 1"));
    assert!(parsed.is_err(), "{parsed:?}");
    assert!(matches!(
        CorsAllow::parse(Some("*")).unwrap(),
        CorsAllow::Any
    ));
    assert!(matches!(
        CorsAllow::parse(Some("  ")).unwrap(),
        CorsAllow::Any
    ));
}

#[tokio::test]
async fn dual_routes_keep_distinct_breach_json() {
    let app = app(python_lock(0.5));
    let (chat_status, _, chat_body) = oneshot(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/chat")
            .header("content-type", "application/json")
            .header("origin", "https://studio.example")
            .body(Body::from(
                r#"{"messages":[{"role":"user","content":"BREACH"}],"domain_id":"python"}"#,
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(chat_status, StatusCode::FORBIDDEN, "{chat_body}");
    let chat: serde_json::Value = serde_json::from_str(&chat_body).unwrap();
    assert_eq!(chat["decision"], "BREACH");
    assert_eq!(chat["failing_tier"], "L1_CENTROID_COSINE");
    assert_eq!(chat["l1"]["passed"], false);
    assert!(chat.get("error").is_none(), "{chat_body}");

    let (v1_status, v1_headers, v1_body) = oneshot(
        app,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .header("origin", "https://studio.example")
            .header("x-semantic-domain", "python")
            .body(Body::from(
                r#"{"messages":[{"role":"user","content":"BREACH"}]}"#,
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(v1_status, StatusCode::FORBIDDEN, "{v1_body}");
    assert!(v1_body.contains("SEMANTIC_CONTAINMENT_BREACH"), "{v1_body}");
    assert!(v1_body.contains("L1_CATEGORICAL_COSINE"), "{v1_body}");
    assert!(!v1_body.contains("L1_CENTROID_COSINE"), "{v1_body}");
    assert_eq!(
        v1_headers
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );
}

#[tokio::test]
async fn chat_override_does_not_mutate_the_registry_lock() {
    let (registry, app_state) = state(python_lock(1.1), Arc::new(MockUpstreamForwarder));
    let app = router(app_state);
    let (pass_status, pass_headers, pass_body) = oneshot(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/chat")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"messages":[{"role":"user","content":"hello kernel"}],"domain_id":"python","overrides":{"l1_threshold":0.5}}"#,
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(pass_status, StatusCode::OK, "{pass_body}");
    assert!(
        pass_headers
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .starts_with("text/event-stream"),
        "{pass_headers:?}"
    );
    assert!(pass_body.contains("chat.completion.chunk"), "{pass_body}");

    let stored = registry.get("python").expect("python lock");
    assert_eq!(stored.l1_cosine_threshold.to_bits(), 1.1f32.to_bits());

    let (breach_status, _, breach_body) = oneshot(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/v1/chat")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"messages":[{"role":"user","content":"hello kernel"}],"domain_id":"python"}"#,
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(breach_status, StatusCode::FORBIDDEN, "{breach_body}");
    let breach: serde_json::Value = serde_json::from_str(&breach_body).unwrap();
    assert_eq!(breach["failing_tier"], "L1_CENTROID_COSINE");
    let threshold = breach["l1"]["threshold"].as_f64().expect("threshold") as f32;
    assert_eq!(threshold.to_bits(), 1.1f32.to_bits());
}

struct LaneEcho {
    current: Arc<AtomicUsize>,
    max_inflight: Arc<AtomicUsize>,
}

#[async_trait]
impl UpstreamLlmForwarder for LaneEcho {
    async fn forward_chat_completion(
        &self,
        _headers: &HeaderMap,
        raw_body: Bytes,
    ) -> Result<Response, String> {
        let payload: serde_json::Value =
            serde_json::from_slice(&raw_body).map_err(|err| err.to_string())?;
        let lane = payload["messages"]
            .as_array()
            .and_then(|messages| {
                messages
                    .iter()
                    .rev()
                    .find(|message| message["role"] == "user")
            })
            .and_then(|message| message["content"].as_str())
            .unwrap_or("")
            .to_string();
        let current = Arc::clone(&self.current);
        let max_inflight = Arc::clone(&self.max_inflight);
        let first = format!("data: A-{lane}\n\n");
        let second = format!("data: B-{lane}\n\n");
        let stream = stream::unfold(0u8, move |step| {
            let first = first.clone();
            let second = second.clone();
            let current = Arc::clone(&current);
            let max_inflight = Arc::clone(&max_inflight);
            async move {
                match step {
                    0 => {
                        let now = current.fetch_add(1, Ordering::SeqCst) + 1;
                        max_inflight.fetch_max(now, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(30)).await;
                        Some((Ok::<Bytes, std::io::Error>(Bytes::from(first)), 1u8))
                    }
                    1 => {
                        tokio::time::sleep(Duration::from_millis(30)).await;
                        current.fetch_sub(1, Ordering::SeqCst);
                        Some((Ok(Bytes::from(second)), 2u8))
                    }
                    _ => None,
                }
            }
        });
        Response::builder()
            .status(StatusCode::OK)
            .header(axum::http::header::CONTENT_TYPE, "text/event-stream")
            .body(Body::from_stream(stream))
            .map_err(|err| err.to_string())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_chat_sse_lanes_stay_unmixed() {
    const LANES: usize = 16;
    let current = Arc::new(AtomicUsize::new(0));
    let max_inflight = Arc::new(AtomicUsize::new(0));
    let echo = Arc::new(LaneEcho {
        current: Arc::clone(&current),
        max_inflight: Arc::clone(&max_inflight),
    });
    let (_registry, app_state) = state(python_lock(0.5), echo);
    let app = router(app_state);
    let mut tasks = Vec::with_capacity(LANES);
    for lane in 0..LANES {
        let app = app.clone();
        tasks.push(tokio::spawn(async move {
            let body = format!(
                r#"{{"messages":[{{"role":"user","content":"lane-{lane:02}"}}],"domain_id":"python","upstream":{{"provider":"mock"}}}}"#
            );
            let (status, headers, text) = oneshot(
                app,
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await;
            (lane, status, headers, text)
        }));
    }
    for task in tasks {
        let (lane, status, headers, text) = task.await.unwrap();
        assert_eq!(status, StatusCode::OK, "lane-{lane} {text}");
        assert!(
            headers
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
                .starts_with("text/event-stream"),
            "lane-{lane}"
        );
        assert!(
            text.contains(&format!("data: A-lane-{lane:02}\n")),
            "{text}"
        );
        assert!(
            text.contains(&format!("data: B-lane-{lane:02}\n")),
            "{text}"
        );
        let marker_at = text.find(&format!("A-lane-{lane:02}")).unwrap();
        let end_at = text.find(&format!("B-lane-{lane:02}")).unwrap();
        assert!(marker_at < end_at, "{text}");
        for other in 0..LANES {
            if other == lane {
                continue;
            }
            assert!(
                !text.contains(&format!("lane-{other:02}")),
                "lane-{lane:02} mixed with lane-{other:02}: {text}"
            );
        }
    }
    assert!(
        max_inflight.load(Ordering::SeqCst) > 1,
        "streams never overlapped"
    );
}
