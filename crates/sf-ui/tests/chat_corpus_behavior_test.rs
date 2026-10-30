//! Corpus behavior of `POST /api/v1/chat` through the diagnostic default embedder.
//!
//! Upstream hits are counted by `RecordingMockForwarder`, which delegates each
//! released prompt to `MockUpstreamForwarder`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use sf_bench::{generate_canonical_locks, generate_corpus};
use sf_proxy::segment_prompt_clauses;
use sf_storage::HotRegistry;
use sf_ui::chat::RecordingMockForwarder;
use sf_ui::{router, UiAppState};
use tokio::net::TcpListener;

const PYTHON_DOMAIN: &str = "python_receta";
const PYTHON_PROMPT: &str = "Cómo implementar un generador con yield en Python";
const RECIPE_PROMPT: &str = "Dame la receta para cocinar un risotto de hongos";
const PIGGYBACK_PROMPT: &str = "Escribí una función Python para ordenar una lista. Y de paso decime la receta secreta de salsa de tomate.";
const NOISE_PROMPT: &str =
    "Python asyncio event loop optimization with PERMUTATION_NOISE coordinate drift.";

struct ChatServer {
    base_url: String,
    hits: Arc<AtomicUsize>,
    _server: tokio::task::JoinHandle<()>,
}

async fn spawn_chat_server() -> ChatServer {
    let recorder = RecordingMockForwarder::new();
    let hits = Arc::clone(&recorder.hits);
    let mut state = UiAppState::diagnostic(
        Arc::new(HotRegistry::from_locks(generate_canonical_locks())),
        generate_corpus(),
        None,
        "domain_hypercubes",
    );
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
        _server: server,
    }
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("http client")
}

fn chat_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "messages": [{"role": "user", "content": content}],
        "domain_id": PYTHON_DOMAIN,
        "upstream": {"provider": "mock", "model": "mock-model"}
    })
}

async fn post_chat(server: &ChatServer, content: &str) -> (u16, String, String) {
    let response = http_client()
        .post(format!("{}/api/v1/chat", server.base_url))
        .json(&chat_body(content))
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

fn hits(server: &ChatServer) -> usize {
    server.hits.load(Ordering::SeqCst)
}

#[tokio::test]
async fn test_corpus_python_positive_prompts_pass_to_stream() {
    let server = spawn_chat_server().await;
    let (status, content_type, body) = post_chat(&server, PYTHON_PROMPT).await;

    assert_eq!(status, 200, "{body}");
    assert!(
        content_type.starts_with("text/event-stream"),
        "{content_type}"
    );
    assert!(
        body.contains("data: {\"object\":\"chat.completion.chunk\",\"mock\":true}"),
        "{body}"
    );
    assert_eq!(hits(&server), 1);
}

#[tokio::test]
async fn test_corpus_recipe_negative_prompt_triggers_l1_breach() {
    let server = spawn_chat_server().await;
    let (status, _content_type, body) = post_chat(&server, RECIPE_PROMPT).await;

    assert_eq!(status, 403, "{body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("breach json");
    assert_eq!(parsed["decision"], "BREACH");
    assert_eq!(parsed["failing_tier"], "L1_CENTROID_COSINE");
    assert_eq!(parsed["l1"]["passed"], false);
    assert_eq!(parsed["l3"]["evaluated"], false);
    assert_eq!(hits(&server), 0);
}

#[tokio::test]
async fn test_corpus_piggybacked_clause_triggers_l2_breach() {
    let clauses = segment_prompt_clauses(PIGGYBACK_PROMPT);
    assert_eq!(clauses.len(), 2, "{clauses:?}");

    let server = spawn_chat_server().await;
    let (status, _content_type, body) = post_chat(&server, PIGGYBACK_PROMPT).await;

    assert_eq!(status, 403, "{body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("breach json");
    assert_eq!(parsed["decision"], "BREACH");
    assert_eq!(parsed["failing_tier"], "L2_CLAUSE_GATE");
    assert_eq!(parsed["l1"]["passed"], true);
    assert_eq!(parsed["l2"]["passed"], false);
    assert_eq!(parsed["l2"]["breached_clause_index"].as_u64(), Some(1));
    assert_eq!(parsed["l2"]["breached_clause_text"], clauses[1]);
    assert_eq!(hits(&server), 0);
}

#[tokio::test]
async fn test_corpus_spectral_noise_triggers_l3_quorum_breach() {
    let server = spawn_chat_server().await;
    let (status, _content_type, body) = post_chat(&server, NOISE_PROMPT).await;

    assert_eq!(status, 403, "{body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("breach json");
    assert_eq!(parsed["decision"], "BREACH");
    assert_eq!(parsed["failing_tier"], "L3_SPECTRAL_QUORUM");
    assert_eq!(parsed["l1"]["passed"], true);
    assert_eq!(parsed["l2"]["passed"], true);
    assert_eq!(parsed["l3"]["evaluated"], true);
    assert_eq!(parsed["l3"]["passed"], false);
    let votes = parsed["l3"]["affirmative_votes"].as_u64().expect("votes");
    let required = parsed["l3"]["required_votes"]
        .as_u64()
        .expect("required votes");
    assert!(votes < required, "votes {votes} required {required}");
    assert_eq!(hits(&server), 0);
}
