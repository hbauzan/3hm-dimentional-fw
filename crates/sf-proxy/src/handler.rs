//! Ingress routes for the gateway daemon.

use crate::chat_api::handle_api_v1_chat;
use crate::cors::CorsAllow;
use crate::embedder::BatchEmbedderClient;
use crate::routing::{evaluate_route, RouteFailure, RouteInput};
use crate::segmenter::segment_prompt_clauses;
use crate::upstream::UpstreamLlmForwarder;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bytes::Bytes;
use serde::Deserialize;
use sf_kernels::L3InspectionMode;
use sf_storage::HotRegistry;
use std::sync::Arc;

pub struct ProxyAppState {
    pub registry: Arc<HotRegistry>,
    pub embedder: Arc<dyn BatchEmbedderClient>,
    pub upstream: Arc<dyn UpstreamLlmForwarder>,
    pub mock_upstream: Arc<dyn UpstreamLlmForwarder>,
    pub default_domain: String,
    pub max_telemetry_coords: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatRequest {
    messages: Vec<OpenAiMessage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: Option<String>,
}

pub fn router(state: Arc<ProxyAppState>) -> axum::Router {
    router_with_cors(state, CorsAllow::Any)
}

pub fn router_with_cors(state: Arc<ProxyAppState>, cors: CorsAllow) -> axum::Router {
    axum::Router::new()
        .route(
            "/v1/chat/completions",
            axum::routing::post(handle_v1_chat_completions),
        )
        .route("/api/v1/chat", axum::routing::post(handle_api_v1_chat))
        .route(
            "/api/v1/upstream/env",
            axum::routing::get(handle_api_v1_upstream_env),
        )
        .layer(cors.layer())
        .with_state(state)
}

pub async fn handle_api_v1_upstream_env() -> Response {
    let catalog = crate::env_config::get_all_providers_env();
    (StatusCode::OK, axum::Json(catalog)).into_response()
}

pub async fn handle_v1_chat_completions(
    State(state): State<Arc<ProxyAppState>>,
    headers: HeaderMap,
    raw_body: Bytes,
) -> Response {
    let parsed: OpenAiChatRequest = match serde_json::from_slice(&raw_body) {
        Ok(req) => req,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": { "type": "invalid_request_error", "message": err.to_string() }
                })),
            )
                .into_response();
        }
    };

    let user_prompt = match parsed
        .messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .and_then(|message| message.content.as_deref())
    {
        Some(text) if !text.trim().is_empty() => text.to_string(),
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": {
                        "type": "ddi_ingress_breach",
                        "code": "EMPTY_USER_PROMPT",
                        "message": "Fail-closed rejection: missing or empty user role content."
                    }
                })),
            )
                .into_response();
        }
    };

    let l3_mode = match headers
        .get("x-ddi-mode")
        .and_then(|value| value.to_str().ok())
    {
        Some("mode_a") => L3InspectionMode::ModeATopK,
        Some("mode_b") => L3InspectionMode::ModeBStrictHypercube,
        _ => L3InspectionMode::ModeCSpectralQuorum,
    };

    let clauses = segment_prompt_clauses(&user_prompt);
    let mut batch_inputs = Vec::with_capacity(1 + clauses.len());
    batch_inputs.push(user_prompt.as_str());
    batch_inputs.extend(clauses.iter().copied());

    let vectors = match state.embedder.embed_batch_f32(&batch_inputs).await {
        Ok(vectors) if vectors.len() == batch_inputs.len() => vectors,
        Ok(_) => {
            return forbidden(
                "EMBEDDER_CARDINALITY_MISMATCH",
                "Fail-closed: embedder returned mismatched batch vector count.",
            );
        }
        Err(err) => {
            return forbidden(
                "EMBEDDER_UNAVAILABLE",
                &format!("Fail-closed: embedding subsystem error: {err}"),
            );
        }
    };

    match evaluate_route(RouteInput {
        registry: &state.registry,
        headers: &headers,
        default_domain: &state.default_domain,
        full_prompt: &vectors[0],
        clause_vecs: &vectors[1..],
        clauses: &clauses,
        l3_mode,
        max_telemetry_coords: state.max_telemetry_coords,
    }) {
        Ok(gate_latency_ns) => {
            match state
                .upstream
                .forward_chat_completion(&headers, raw_body)
                .await
            {
                Ok(mut response) => {
                    let header_map = response.headers_mut();
                    header_map.insert("x-sf-decision", HeaderValue::from_static("PASS"));
                    if let Ok(value) = HeaderValue::from_str(&gate_latency_ns.to_string()) {
                        header_map.insert("x-sf-latency-ns", value);
                    }
                    if let Ok(value) = HeaderValue::from_str(&clauses.len().to_string()) {
                        header_map.insert("x-sf-clauses-evaluated", value);
                    }
                    response
                }
                Err(err) => (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({
                        "error": { "type": "upstream_error", "message": err }
                    })),
                )
                    .into_response(),
            }
        }
        Err(failure) => match *failure {
            RouteFailure::Route(route_err) => forbidden(route_err.status_code, &route_err.message),
            RouteFailure::Breach(envelope) => {
                (StatusCode::FORBIDDEN, Json(*envelope)).into_response()
            }
            RouteFailure::Pairwise(envelope) => {
                (StatusCode::FORBIDDEN, Json(*envelope)).into_response()
            }
        },
    }
}

fn forbidden(code: &'static str, message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "error": {
                "type": "ddi_ingress_breach",
                "code": code,
                "message": message
            }
        })),
    )
        .into_response()
}
