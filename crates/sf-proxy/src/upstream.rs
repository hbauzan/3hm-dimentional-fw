//! Upstream LLM seam. Slice 1 returns a static 200. SSE passthrough lands in Slice 4.

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use bytes::Bytes;
use futures_util::TryStreamExt;

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
        let chunks = tokio_stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::from(
            "data: {\"object\":\"chat.completion.chunk\",\"mock\":true}\n\n",
        ))]);
        Response::builder()
            .status(StatusCode::OK)
            .header(axum::http::header::CONTENT_TYPE, "text/event-stream")
            .body(Body::from_stream(chunks))
            .map_err(|err| err.to_string())
    }
}

/// Forwards the original body and streams the upstream bytes without buffering the SSE payload.
pub struct ReqwestChatForwarder {
    client: reqwest::Client,
    base_url: String,
}

impl ReqwestChatForwarder {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl UpstreamLlmForwarder for ReqwestChatForwarder {
    async fn forward_chat_completion(
        &self,
        headers: &HeaderMap,
        raw_body: Bytes,
    ) -> Result<Response, String> {
        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );
        let mut request = self.client.post(url).body(raw_body);
        if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
            request = request.header(axum::http::header::AUTHORIZATION, auth);
        }
        if let Some(content_type) = headers.get(axum::http::header::CONTENT_TYPE) {
            request = request.header(axum::http::header::CONTENT_TYPE, content_type);
        }
        let upstream = request
            .send()
            .await
            .map_err(|err| format!("upstream send failed: {err}"))?;
        let status = upstream.status();
        let content_type = upstream
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/json")
            .to_string();
        let stream = upstream.bytes_stream().map_err(std::io::Error::other);
        Response::builder()
            .status(status.as_u16())
            .header(axum::http::header::CONTENT_TYPE, content_type)
            .body(Body::from_stream(stream))
            .map_err(|err| err.to_string())
    }
}
