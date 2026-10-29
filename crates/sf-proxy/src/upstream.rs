//! Upstream LLM seam. Slice 1 returns a static 200. SSE passthrough lands in Slice 4.

use async_trait::async_trait;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bytes::Bytes;

#[async_trait]
pub trait UpstreamLlmForwarder: Send + Sync {
    async fn forward_chat_completion(
        &self,
        headers: &HeaderMap,
        raw_body: Bytes,
    ) -> Result<Response, String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MockUpstreamForwarder;

#[async_trait]
impl UpstreamLlmForwarder for MockUpstreamForwarder {
    async fn forward_chat_completion(
        &self,
        _headers: &HeaderMap,
        _raw_body: Bytes,
    ) -> Result<Response, String> {
        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "object": "chat.completion",
                "mock": true
            })),
        )
            .into_response())
    }
}
