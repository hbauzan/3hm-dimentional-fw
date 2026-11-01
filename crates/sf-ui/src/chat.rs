//! `POST /api/v1/chat` for the embedded studio.
//! The cascade, breach DTO, and upstream selection live in `sf-proxy`.
//! This handler adds the studio embedder and the corpus fallback for lock lookup.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bytes::Bytes;
use sf_kernels::{AlignedVec1024, DomainHypercubeLock};
use sf_proxy::{
    apply_chat_overrides, chat_upstream_body, evaluate_chat_cascade, evaluate_pairwise_containment,
    latest_chat_user_text, pairwise_breach_envelope, resolve_chat_upstream, segment_prompt_clauses,
    ChatRequestDto, ChatUpstream, MockUpstreamForwarder, UpstreamLlmForwarder,
};

pub use crate::embedder::{
    CalibratedSemanticEmbedder, DeterministicPromptEmbedder, PromptEmbedder,
};
use crate::handlers::UiAppState;

/// Mock forwarder that records how many times the cascade released a prompt.
pub struct RecordingMockForwarder {
    pub hits: Arc<AtomicUsize>,
    pub last_body: Arc<Mutex<Bytes>>,
}

impl RecordingMockForwarder {
    pub fn new() -> Self {
        Self {
            hits: Arc::new(AtomicUsize::new(0)),
            last_body: Arc::new(Mutex::new(Bytes::new())),
        }
    }
}

impl Default for RecordingMockForwarder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UpstreamLlmForwarder for RecordingMockForwarder {
    async fn forward_chat_completion(
        &self,
        headers: &HeaderMap,
        raw_body: Bytes,
    ) -> Result<Response, String> {
        self.hits.fetch_add(1, Ordering::SeqCst);
        {
            let mut slot = self
                .last_body
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *slot = raw_body.clone();
        }
        MockUpstreamForwarder
            .forward_chat_completion(headers, raw_body)
            .await
    }
}

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(serde_json::json!({ "error": message.into() }))).into_response()
}

fn resolve_lock(
    state: &UiAppState,
    domain_id: Option<&str>,
) -> Result<DomainHypercubeLock, (StatusCode, String)> {
    let registry_locks = state.registry.locks();
    if let Some(id) = domain_id {
        if let Some(lock) = registry_locks.iter().find(|lock| lock.domain_id == id) {
            return Ok((**lock).clone());
        }
        if let Some(lock) = state.corpus.locks.iter().find(|lock| lock.domain_id == id) {
            return Ok(lock.clone());
        }
        return Err((StatusCode::NOT_FOUND, format!("unknown domain_id '{id}'")));
    }
    if let Some(lock) = registry_locks.first() {
        return Ok((**lock).clone());
    }
    if let Some(lock) = state.corpus.locks.first() {
        return Ok(lock.clone());
    }
    Err((
        StatusCode::SERVICE_UNAVAILABLE,
        "no domain lock is loaded".to_string(),
    ))
}

pub async fn handle_chat(
    State(state): State<Arc<UiAppState>>,
    Json(payload): Json<ChatRequestDto>,
) -> Response {
    let Some(user_text) = latest_chat_user_text(&payload.messages) else {
        return json_error(StatusCode::BAD_REQUEST, "missing user message");
    };
    let user_text = user_text.to_string();

    let mut lock = match resolve_lock(&state, payload.domain_id.as_deref()) {
        Ok(lock) => lock,
        Err((status, message)) => return json_error(status, message),
    };
    if let Some(overrides) = payload.overrides.as_ref() {
        lock = apply_chat_overrides(&lock, overrides);
    }

    let prompt_vec = state.prompt_embedder.embed(&user_text, &lock);
    let clauses = segment_prompt_clauses(&user_text);
    if clauses.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "prompt produced no clauses");
    }
    let clause_vecs: Vec<AlignedVec1024> = clauses
        .iter()
        .copied()
        .map(|clause| state.prompt_embedder.embed(clause, &lock))
        .collect();

    let gate_dto = match evaluate_chat_cascade(&prompt_vec, &clauses, &clause_vecs, &lock) {
        Ok(dto) => dto,
        Err(dto) => return (StatusCode::FORBIDDEN, Json(*dto)).into_response(),
    };
    let pairs = state.registry.pairwise_locks_for(&lock.domain_id);
    if let Err(breach) =
        evaluate_pairwise_containment(&prompt_vec, &clause_vecs, &lock.domain_id, &pairs)
    {
        return (
            StatusCode::FORBIDDEN,
            Json(pairwise_breach_envelope(breach)),
        )
            .into_response();
    }

    let upstream = match resolve_chat_upstream(payload.upstream.as_ref()) {
        Ok(upstream) => upstream,
        Err(reject) => return json_error(reject.status, reject.message),
    };
    let bytes = match chat_upstream_body(upstream.model(), &payload.messages) {
        Ok(bytes) => bytes,
        Err(err) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, err),
    };

    let forwarded = match upstream {
        ChatUpstream::Mock { .. } => {
            state
                .upstream
                .forward_chat_completion(&HeaderMap::new(), bytes)
                .await
        }
        ChatUpstream::Http {
            base_url, api_key, ..
        } => {
            state
                .http_forwarder
                .forward_dynamic(&base_url, api_key.as_deref(), bytes)
                .await
        }
    };

    match forwarded {
        Ok(mut response) => {
            let headers = response.headers_mut();
            headers.insert("x-sf-decision", HeaderValue::from_static("PASS"));
            if let Ok(value) = HeaderValue::from_str(&clauses.len().to_string()) {
                headers.insert("x-sf-clauses-evaluated", value);
            }
            if let Ok(json_str) = serde_json::to_string(&gate_dto) {
                if let Ok(val) = HeaderValue::from_str(&json_str) {
                    headers.insert("x-sf-telemetry", val);
                }
            }
            response
        }
        Err(err) => json_error(StatusCode::BAD_GATEWAY, err),
    }
}
