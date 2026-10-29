//! TEI gRPC and HTTP embedders. Every gRPC request sets `normalize = true`.

use crate::embedder::BatchEmbedderClient;
use crate::normalize::{validate_and_normalize_f32_slice, NormalizeError};
use async_trait::async_trait;
use sf_kernels::AlignedVec1024;

pub mod pb {
    tonic::include_proto!("tei.v1");
}

pub fn embed_request(input: &str) -> pb::EmbedRequest {
    pb::EmbedRequest {
        inputs: input.to_string(),
        truncate: false,
        normalize: Some(true),
        truncation_direction: pb::TruncationDirection::Right as i32,
        prompt_name: None,
        dimensions: None,
    }
}

pub struct TeiGrpcEmbedder {
    client: pb::embed_client::EmbedClient<tonic::transport::Channel>,
}

impl TeiGrpcEmbedder {
    pub async fn connect(url: impl AsRef<str>) -> Result<Self, String> {
        let client = pb::embed_client::EmbedClient::connect(url.as_ref().to_string())
            .await
            .map_err(|err| err.to_string())?;
        Ok(Self { client })
    }
}

#[async_trait]
impl BatchEmbedderClient for TeiGrpcEmbedder {
    async fn embed_batch_f32(&self, texts: &[&str]) -> Result<Vec<AlignedVec1024>, String> {
        let mut client = self.client.clone();
        let requests: Vec<_> = texts
            .iter()
            .map(|text| {
                let request = embed_request(text);
                if request.normalize != Some(true) {
                    return Err("fail-closed: EmbedRequest.normalize is not Some(true)".to_string());
                }
                Ok(request)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let response = client
            .embed_stream(tokio_stream::iter(requests))
            .await
            .map_err(|err| format!("fail-closed: TEI EmbedStream: {err}"))?;
        let mut inbound = response.into_inner();
        let mut vectors = Vec::with_capacity(texts.len());
        while let Some(message) = inbound
            .message()
            .await
            .map_err(|err| format!("fail-closed: TEI stream: {err}"))?
        {
            vectors.push(normalize_or_string(&message.embeddings)?);
        }
        if vectors.len() != texts.len() {
            return Err("fail-closed: TEI returned a mismatched embedding count".to_string());
        }
        Ok(vectors)
    }
}

pub struct TeiHttpEmbedder {
    client: reqwest::Client,
    base_url: String,
}

impl TeiHttpEmbedder {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl BatchEmbedderClient for TeiHttpEmbedder {
    async fn embed_batch_f32(&self, texts: &[&str]) -> Result<Vec<AlignedVec1024>, String> {
        let url = format!("{}/embed", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .json(&serde_json::json!({
                "inputs": texts,
                "normalize": true
            }))
            .send()
            .await
            .map_err(|err| format!("fail-closed: TEI HTTP /embed: {err}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "fail-closed: TEI HTTP status {}",
                response.status()
            ));
        }
        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|err| format!("fail-closed: TEI HTTP body: {err}"))?;
        let rows = payload
            .as_array()
            .or_else(|| payload.get("embeddings").and_then(|value| value.as_array()))
            .ok_or_else(|| "fail-closed: TEI HTTP payload is not an embedding list".to_string())?;
        let mut vectors = Vec::with_capacity(rows.len());
        for row in rows {
            let nums = row
                .as_array()
                .ok_or_else(|| "fail-closed: TEI HTTP row is not an array".to_string())?;
            let mut raw = Vec::with_capacity(nums.len());
            for number in nums {
                let value = number.as_f64().ok_or_else(|| {
                    "fail-closed: TEI HTTP coordinate is not a number".to_string()
                })?;
                raw.push(value as f32);
            }
            vectors.push(normalize_or_string(&raw)?);
        }
        Ok(vectors)
    }
}

fn normalize_or_string(values: &[f32]) -> Result<AlignedVec1024, String> {
    validate_and_normalize_f32_slice(values).map_err(|err: NormalizeError| err.to_string())
}
