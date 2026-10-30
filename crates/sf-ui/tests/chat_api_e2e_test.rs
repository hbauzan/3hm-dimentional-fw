//! `POST /api/v1/chat` behavior: SSE on pass, 403 telemetry on breach, live L1 override.
//!
//! The studio binary has no TEI client. These tests inject the prompt vector so the
//! cascade math is what the assertions measure.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use sf_bench::{generate_canonical_locks, generate_corpus};
use sf_kernels::{
    cosine_similarity_1024, evaluate_l3_gate, AlignedVec1024, DomainHypercubeLock,
    L3InspectionMode, DIM,
};
use sf_storage::HotRegistry;
use sf_ui::chat::{PromptEmbedder, RecordingMockForwarder};
use sf_ui::{router, UiAppState};
use tokio::net::TcpListener;

const PYTHON_DOMAIN: &str = "python_receta";
const RECIPE: &str = "Dame la receta para cocinar un risotto de hongos";

struct FixedEmbedder(AlignedVec1024);

impl PromptEmbedder for FixedEmbedder {
    fn embed(&self, _text: &str, _lock: &DomainHypercubeLock) -> AlignedVec1024 {
        self.0
    }
}

struct ChatServer {
    base_url: String,
    hits: Arc<AtomicUsize>,
    last_body: Arc<Mutex<Bytes>>,
    _server: tokio::task::JoinHandle<()>,
}

fn python_lock() -> DomainHypercubeLock {
    generate_canonical_locks()
        .into_iter()
        .find(|lock| lock.domain_id == PYTHON_DOMAIN)
        .expect("python_receta lock")
}

fn orthogonal_to(lock: &DomainHypercubeLock) -> AlignedVec1024 {
    let spike = lock
        .centroid
        .data
        .iter()
        .position(|value| *value != 0.0)
        .expect("centroid spike");
    let mut vector = AlignedVec1024::zeros();
    let other = if spike + 1 < DIM { spike + 1 } else { 0 };
    vector.data[other] = 1.0;
    vector
}

/// In-box vector whose cosine with the centroid sits in `[0.50, 0.75)`.
fn mid_cosine_in_box(lock: &DomainHypercubeLock) -> AlignedVec1024 {
    let mut vector = AlignedVec1024::zeros();
    for dim in 0..DIM {
        vector.data[dim] = if lock.centroid.data[dim] != 0.0 {
            lock.lo_dilated.data[dim]
        } else {
            lock.hi_dilated.data[dim]
        };
    }
    vector
}

async fn spawn_chat_server(embedder: Arc<dyn PromptEmbedder>) -> ChatServer {
    let recorder = RecordingMockForwarder::new();
    let hits = Arc::clone(&recorder.hits);
    let last_body = Arc::clone(&recorder.last_body);
    let mut state = UiAppState::diagnostic(
        Arc::new(HotRegistry::from_locks(generate_canonical_locks())),
        generate_corpus(),
        None,
        "domain_hypercubes",
    );
    state.prompt_embedder = embedder;
    state.upstream = Arc::new(recorder);
    let app = router(Arc::new(state));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{addr}");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve test server");
    });
    ChatServer {
        base_url,
        hits,
        last_body,
        _server: server,
    }
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("http client")
}

async fn post_chat(server: &ChatServer, payload: serde_json::Value) -> (u16, String, String) {
    let response = http_client()
        .post(format!("{}/api/v1/chat", server.base_url))
        .json(&payload)
        .send()
        .await
        .expect("POST /api/v1/chat");
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = response.text().await.expect("response body");
    (status, content_type, body)
}

fn chat_body(content: &str, l1_threshold: Option<f32>, api_key: Option<&str>) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "messages": [{"role": "user", "content": content}],
        "domain_id": PYTHON_DOMAIN,
        "upstream": {"provider": "mock", "model": "mock-model"}
    });
    if let Some(api_key) = api_key {
        payload["upstream"]["api_key"] = serde_json::json!(api_key);
    }
    if let Some(threshold) = l1_threshold {
        payload["overrides"] = serde_json::json!({ "l1_threshold": threshold });
    }
    payload
}

#[tokio::test]
async fn test_chat_pass_streams_sse_with_mock_upstream() {
    let lock = python_lock();
    let server = spawn_chat_server(Arc::new(FixedEmbedder(lock.centroid))).await;
    let (status, content_type, body) = post_chat(
        &server,
        chat_body(
            "How do I write a Python generator?",
            Some(0.2),
            Some("sk-test-secret"),
        ),
    )
    .await;

    assert_eq!(status, 200, "{body}");
    assert!(
        content_type.starts_with("text/event-stream"),
        "{content_type}"
    );
    assert!(
        body.contains("data: {\"object\":\"chat.completion.chunk\",\"mock\":true}"),
        "{body}"
    );
    assert_eq!(server.hits.load(Ordering::SeqCst), 1);

    let recorded = server
        .last_body
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let wire: serde_json::Value = serde_json::from_slice(&recorded).expect("openai body");
    assert_eq!(wire["stream"], true);
    assert_eq!(wire["model"], "mock-model");
    assert_eq!(
        wire["messages"][0]["content"],
        "How do I write a Python generator?"
    );
    let raw = String::from_utf8(recorded.to_vec()).expect("utf8 body");
    assert!(
        !raw.contains("sk-test-secret"),
        "api key leaked into upstream body: {raw}"
    );
    assert!(!raw.contains("domain_id"), "{raw}");
    assert!(!raw.contains("l1_threshold"), "{raw}");
}

#[tokio::test]
async fn test_chat_breach_returns_403_structured_telemetry() {
    let lock = python_lock();
    let vector = orthogonal_to(&lock);
    let cosine = cosine_similarity_1024(&vector, &lock.centroid);
    assert!(
        cosine < lock.l1_cosine_threshold,
        "recipe stand-in cosine {cosine} must miss the python lock"
    );

    let server = spawn_chat_server(Arc::new(FixedEmbedder(vector))).await;
    let (status, content_type, body) = post_chat(&server, chat_body(RECIPE, None, None)).await;

    assert_eq!(status, 403, "{body}");
    assert!(
        content_type.starts_with("application/json"),
        "{content_type}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("breach json");
    assert_eq!(parsed["decision"], "BREACH");
    assert_eq!(parsed["failing_tier"], "L1_CENTROID_COSINE");
    assert_eq!(parsed["l1"]["passed"], false);
    let reported = parsed["l1"]["cosine"].as_f64().expect("cosine");
    assert!((reported - f64::from(cosine)).abs() < 1e-5, "{reported}");
    let threshold = parsed["l1"]["threshold"].as_f64().expect("threshold");
    assert!((threshold - f64::from(lock.l1_cosine_threshold)).abs() < 1e-6);
    assert!(
        parsed["l2"].get("breached_clause_index").is_none(),
        "{body}"
    );
    assert_eq!(parsed["l3"]["evaluated"], false);
    assert!(parsed["l3"].get("passed").is_none(), "{body}");
    assert_eq!(server.hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_chat_runtime_overrides_alter_gate_decision() {
    let lock = python_lock();
    let vector = mid_cosine_in_box(&lock);
    let cosine = cosine_similarity_1024(&vector, &lock.centroid);
    assert!(
        (0.50..0.75).contains(&cosine),
        "override probe cosine {cosine} must sit between 0.50 and 0.75"
    );
    let l3 = evaluate_l3_gate(&vector, &lock, L3InspectionMode::ModeCSpectralQuorum, 0);
    assert!(
        l3.passed,
        "in-box probe failed quorum {}/{}",
        l3.affirmative_votes, l3.required_votes
    );

    let server = spawn_chat_server(Arc::new(FixedEmbedder(vector))).await;
    let (status, _content_type, body) =
        post_chat(&server, chat_body("threshold probe", Some(0.75), None)).await;
    assert_eq!(status, 403, "{body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("breach json");
    assert_eq!(parsed["decision"], "BREACH");
    assert_eq!(parsed["failing_tier"], "L1_CENTROID_COSINE");
    assert_eq!(parsed["l1"]["passed"], false);
    let threshold = parsed["l1"]["threshold"].as_f64().expect("threshold");
    assert!((threshold - 0.75).abs() < 1e-6, "{threshold}");
    assert_eq!(server.hits.load(Ordering::SeqCst), 0);

    let (status, content_type, body) =
        post_chat(&server, chat_body("threshold probe", Some(0.50), None)).await;
    assert_eq!(status, 200, "{body}");
    assert!(
        content_type.starts_with("text/event-stream"),
        "{content_type}"
    );
    assert!(body.contains("chat.completion.chunk"), "{body}");
    assert_eq!(server.hits.load(Ordering::SeqCst), 1);
}
