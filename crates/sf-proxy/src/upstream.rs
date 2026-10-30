//! Upstream LLM seam. Bytes are streamed through to the caller.

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

    /// Stream one OpenAI-compatible chat completion.
    ///
    /// `base_url` is joined per request. A bearer token, when present, is sent as
    /// `Authorization` and is not written into the body.
    pub async fn forward_dynamic(
        &self,
        base_url: &str,
        bearer_token: Option<&str>,
        raw_body: Bytes,
    ) -> Result<Response, String> {
        let url = resolve_chat_completions_url(base_url);
        let mut request = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(raw_body);
        if let Some(token) = bearer_token
            .map(str::trim)
            .filter(|token| !token.is_empty())
        {
            let mut value = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| "api_key is not a valid Authorization header value".to_string())?;
            value.set_sensitive(true);
            request = request.header(reqwest::header::AUTHORIZATION, value);
        }
        send_and_stream(request, "text/event-stream").await
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
        send_and_stream(request, "application/json").await
    }
}

/// Join an OpenAI-compatible base with the chat-completions path.
///
/// A base that already ends in `/v1` or `/openai` keeps that prefix (`…/chat/completions`).
/// A host with no version prefix receives `/v1/chat/completions`.
fn resolve_chat_completions_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else if base.ends_with("/v1") || base.ends_with("/openai") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

async fn send_and_stream(
    request: reqwest::RequestBuilder,
    default_content_type: &'static str,
) -> Result<Response, String> {
    let upstream = request
        .send()
        .await
        .map_err(|err| format!("upstream send failed: {err}"))?;
    let status = upstream.status();
    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or(default_content_type)
        .to_string();
    let stream = upstream.bytes_stream().map_err(std::io::Error::other);
    Response::builder()
        .status(status.as_u16())
        .header(axum::http::header::CONTENT_TYPE, content_type)
        .body(Body::from_stream(stream))
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::time::timeout;

    #[test]
    fn joins_versioned_host_and_gemini_bases() {
        assert_eq!(
            resolve_chat_completions_url("https://api.deepseek.com/v1"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            resolve_chat_completions_url("https://api.deepseek.com/"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            resolve_chat_completions_url("https://generativelanguage.googleapis.com/v1beta/openai"),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
        assert_eq!(
            resolve_chat_completions_url("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[tokio::test]
    async fn forward_dynamic_posts_bearer_without_putting_it_in_the_body() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let request = read_http11(&mut socket).await;
            let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: 6\r\nconnection: close\r\n\r\ndata: x";
            socket.write_all(response.as_bytes()).await.expect("write");
            request
        });

        let forwarder = ReqwestChatForwarder::new("http://127.0.0.1");
        let response = timeout(
            Duration::from_secs(3),
            forwarder.forward_dynamic(
                &format!("http://{addr}/v1"),
                Some("super-secret"),
                Bytes::from_static(br#"{"stream":true,"model":"mock"}"#),
            ),
        )
        .await
        .expect("forward timeout")
        .expect("forward");
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.starts_with("text/event-stream"),
            "{content_type}"
        );

        let request = timeout(Duration::from_secs(3), server)
            .await
            .expect("server join timeout")
            .expect("server task");
        let lowered = request.to_ascii_lowercase();
        assert!(
            lowered.starts_with("post /v1/chat/completions http/1.1"),
            "{request}"
        );
        assert!(
            lowered.contains("authorization: bearer super-secret"),
            "{request}"
        );
        assert!(
            request.contains(r#"{"stream":true,"model":"mock"}"#),
            "{request}"
        );
        assert_eq!(request.matches("super-secret").count(), 1, "{request}");
    }

    async fn read_http11(socket: &mut tokio::net::TcpStream) -> String {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 2048];
        loop {
            let n = timeout(Duration::from_secs(2), socket.read(&mut tmp))
                .await
                .expect("read timeout")
                .expect("read");
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(header_end) = find_subsequence(&buf, b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&buf[..header_end]);
                let length = content_length(&headers).unwrap_or(0);
                if buf.len() >= header_end + 4 + length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn content_length(headers: &str) -> Option<usize> {
        headers.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
    }
}
