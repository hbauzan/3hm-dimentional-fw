//! `POST /api/v1/chat`: three-tier gate, then unbuffered upstream SSE or HTTP 403.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sf_kernels::{
    cosine_similarity_1024, evaluate_l3_gate, AlignedVec1024, DomainHypercubeLock,
    L3InspectionMode, DIM,
};
use sf_proxy::{segment_prompt_clauses, MockUpstreamForwarder, UpstreamLlmForwarder};

pub use crate::embedder::{
    CalibratedSemanticEmbedder, DeterministicPromptEmbedder, PromptEmbedder,
};
use crate::handlers::UiAppState;

const DECISION_BREACH: &str = "BREACH";
const TIER_L1: &str = "L1_CENTROID_COSINE";
const TIER_L2: &str = "L2_CLAUSE_GATE";
const TIER_L3: &str = "L3_SPECTRAL_QUORUM";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatOverridesDto {
    pub l1_threshold: Option<f32>,
    pub l2_threshold: Option<f32>,
    pub quorum_k: Option<u16>,
    pub epsilon: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamConfigDto {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequestDto {
    pub messages: Vec<ChatMessageDto>,
    pub domain_id: Option<String>,
    pub overrides: Option<ChatOverridesDto>,
    pub upstream: Option<UpstreamConfigDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tier1BreachDto {
    pub passed: bool,
    pub cosine: f32,
    pub threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tier2BreachDto {
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breached_clause_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breached_clause_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clause_cosine: Option<f32>,
    pub threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tier3BreachDto {
    pub evaluated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affirmative_votes: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_votes: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBreachDto {
    pub decision: String,
    pub failing_tier: String,
    pub l1: Tier1BreachDto,
    pub l2: Tier2BreachDto,
    pub l3: Tier3BreachDto,
}

#[derive(Serialize)]
struct OpenAiStreamRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessageDto],
    stream: bool,
}

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

enum CascadeDecision {
    Pass,
    Breach(ChatBreachDto),
}

enum UpstreamTransport {
    Mock,
    Http {
        base_url: String,
        api_key: Option<String>,
    },
}

struct ResolvedUpstream {
    model: String,
    transport: UpstreamTransport,
}

/// Allowlist polarity `0` passes when `score >= threshold`.
/// Denylist polarity `1` passes when `score < threshold`.
fn tier_passes(polarity: u8, score: f32, threshold: f32) -> bool {
    if polarity == 0 {
        score >= threshold
    } else {
        score < threshold
    }
}

struct ChatFailure {
    status: StatusCode,
    message: String,
}

fn failure(status: StatusCode, message: impl Into<String>) -> ChatFailure {
    ChatFailure {
        status,
        message: message.into(),
    }
}

impl ChatFailure {
    fn into_response(self) -> Response {
        json_error(self.status, self.message)
    }
}

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(serde_json::json!({ "error": message.into() }))).into_response()
}

fn breach_response(dto: ChatBreachDto) -> Response {
    (StatusCode::FORBIDDEN, Json(dto)).into_response()
}

fn latest_user_text(messages: &[ChatMessageDto]) -> Option<&str> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.trim())
        .filter(|text| !text.is_empty())
}

fn resolve_lock(
    state: &UiAppState,
    domain_id: Option<&str>,
) -> Result<DomainHypercubeLock, ChatFailure> {
    let registry_locks = state.registry.locks();
    if let Some(id) = domain_id {
        if let Some(lock) = registry_locks.iter().find(|lock| lock.domain_id == id) {
            return Ok((**lock).clone());
        }
        if let Some(lock) = state.corpus.locks.iter().find(|lock| lock.domain_id == id) {
            return Ok(lock.clone());
        }
        return Err(failure(
            StatusCode::NOT_FOUND,
            format!("unknown domain_id '{id}'"),
        ));
    }
    if let Some(lock) = registry_locks.first() {
        return Ok((**lock).clone());
    }
    if let Some(lock) = state.corpus.locks.first() {
        return Ok(lock.clone());
    }
    Err(failure(
        StatusCode::SERVICE_UNAVAILABLE,
        "no domain lock is loaded",
    ))
}

fn apply_overrides(
    lock: &DomainHypercubeLock,
    overrides: &ChatOverridesDto,
) -> DomainHypercubeLock {
    let mut next = lock.clone();
    if let Some(threshold) = overrides.l1_threshold {
        next.l1_cosine_threshold = threshold;
    }
    if let Some(threshold) = overrides.l2_threshold {
        next.l2_clause_threshold = threshold;
    }
    if let Some(quorum_k) = overrides.quorum_k {
        next.quorum_k = quorum_k;
    }
    if let Some(epsilon) = overrides.epsilon {
        let previous = next.epsilon;
        for dim in 0..DIM {
            let lo = next.lo_dilated.data[dim] + previous;
            let hi = next.hi_dilated.data[dim] - previous;
            next.lo_dilated.data[dim] = lo - epsilon;
            next.hi_dilated.data[dim] = hi + epsilon;
        }
        next.epsilon = epsilon;
    }
    next
}

fn l1_dto(passed: bool, cosine: f32, threshold: f32) -> Tier1BreachDto {
    Tier1BreachDto {
        passed,
        cosine,
        threshold,
    }
}

fn l2_clear(threshold: f32) -> Tier2BreachDto {
    Tier2BreachDto {
        passed: true,
        breached_clause_index: None,
        breached_clause_text: None,
        clause_cosine: None,
        threshold,
    }
}

fn l3_skipped() -> Tier3BreachDto {
    Tier3BreachDto {
        evaluated: false,
        passed: None,
        affirmative_votes: None,
        required_votes: None,
    }
}

fn breach(
    failing_tier: &str,
    l1: Tier1BreachDto,
    l2: Tier2BreachDto,
    l3: Tier3BreachDto,
) -> ChatBreachDto {
    ChatBreachDto {
        decision: DECISION_BREACH.to_string(),
        failing_tier: failing_tier.to_string(),
        l1,
        l2,
        l3,
    }
}

fn evaluate_cascade(
    prompt_vec: &AlignedVec1024,
    clauses: &[&str],
    clause_vecs: &[AlignedVec1024],
    lock: &DomainHypercubeLock,
) -> CascadeDecision {
    let l1_cosine = cosine_similarity_1024(prompt_vec, &lock.centroid);
    let l1_threshold = lock.l1_cosine_threshold;
    if !tier_passes(lock.polarity, l1_cosine, l1_threshold) {
        return CascadeDecision::Breach(breach(
            TIER_L1,
            l1_dto(false, l1_cosine, l1_threshold),
            l2_clear(lock.l2_clause_threshold),
            l3_skipped(),
        ));
    }

    let l2_threshold = lock.l2_clause_threshold;
    for (idx, (clause, clause_vec)) in clauses.iter().zip(clause_vecs.iter()).enumerate() {
        let clause_cosine = cosine_similarity_1024(clause_vec, &lock.centroid);
        if !tier_passes(lock.polarity, clause_cosine, l2_threshold) {
            return CascadeDecision::Breach(breach(
                TIER_L2,
                l1_dto(true, l1_cosine, l1_threshold),
                Tier2BreachDto {
                    passed: false,
                    breached_clause_index: Some(idx),
                    breached_clause_text: Some((*clause).to_string()),
                    clause_cosine: Some(clause_cosine),
                    threshold: l2_threshold,
                },
                l3_skipped(),
            ));
        }
    }

    for clause_vec in clause_vecs {
        let l3 = evaluate_l3_gate(clause_vec, lock, L3InspectionMode::ModeCSpectralQuorum, 0);
        if !l3.passed {
            return CascadeDecision::Breach(breach(
                TIER_L3,
                l1_dto(true, l1_cosine, l1_threshold),
                l2_clear(l2_threshold),
                Tier3BreachDto {
                    evaluated: true,
                    passed: Some(false),
                    affirmative_votes: Some(l3.affirmative_votes),
                    required_votes: Some(l3.required_votes),
                },
            ));
        }
    }

    CascadeDecision::Pass
}

fn nonempty(value: Option<&str>, default_value: &str) -> String {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(default_value)
        .to_string()
}

fn bearer_token(raw: Option<&str>) -> Result<Option<String>, ChatFailure> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let token = raw.trim();
    if token.is_empty() {
        return Ok(None);
    }
    if HeaderValue::from_str(&format!("Bearer {token}")).is_err() {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            "api_key contains characters that cannot be sent as an Authorization header",
        ));
    }
    Ok(Some(token.to_string()))
}

fn http_upstream(
    cfg: &UpstreamConfigDto,
    default_base: &str,
    default_model: &str,
) -> Result<ResolvedUpstream, ChatFailure> {
    Ok(ResolvedUpstream {
        model: nonempty(cfg.model.as_deref(), default_model),
        transport: UpstreamTransport::Http {
            base_url: nonempty(cfg.base_url.as_deref(), default_base),
            api_key: bearer_token(cfg.api_key.as_deref())?,
        },
    })
}

fn resolve_upstream(cfg: Option<&UpstreamConfigDto>) -> Result<ResolvedUpstream, ChatFailure> {
    let Some(cfg) = cfg else {
        return Ok(ResolvedUpstream {
            model: "mock".to_string(),
            transport: UpstreamTransport::Mock,
        });
    };
    match cfg.provider.as_deref().unwrap_or("mock") {
        "mock" => Ok(ResolvedUpstream {
            model: nonempty(cfg.model.as_deref(), "mock"),
            transport: UpstreamTransport::Mock,
        }),
        "deepseek" => http_upstream(cfg, "https://api.deepseek.com/v1", "deepseek-chat"),
        "openai" => http_upstream(cfg, "https://api.openai.com/v1", "gpt-4o-mini"),
        "gemini" => http_upstream(
            cfg,
            "https://generativelanguage.googleapis.com/v1beta/openai",
            "gemini-2.0-flash",
        ),
        other => Err(failure(
            StatusCode::BAD_REQUEST,
            format!("unsupported upstream provider '{other}'"),
        )),
    }
}

pub async fn handle_chat(
    State(state): State<Arc<UiAppState>>,
    Json(payload): Json<ChatRequestDto>,
) -> Response {
    let Some(user_text) = latest_user_text(&payload.messages) else {
        return json_error(StatusCode::BAD_REQUEST, "missing user message");
    };
    let user_text = user_text.to_string();

    let mut lock = match resolve_lock(&state, payload.domain_id.as_deref()) {
        Ok(lock) => lock,
        Err(reject) => return reject.into_response(),
    };
    if let Some(overrides) = payload.overrides.as_ref() {
        lock = apply_overrides(&lock, overrides);
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

    if let CascadeDecision::Breach(dto) =
        evaluate_cascade(&prompt_vec, &clauses, &clause_vecs, &lock)
    {
        return breach_response(dto);
    }

    let upstream = match resolve_upstream(payload.upstream.as_ref()) {
        Ok(upstream) => upstream,
        Err(reject) => return reject.into_response(),
    };
    let body = OpenAiStreamRequest {
        model: &upstream.model,
        messages: &payload.messages,
        stream: true,
    };
    let bytes = match serde_json::to_vec(&body) {
        Ok(bytes) => Bytes::from(bytes),
        Err(err) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    };

    let forwarded = match upstream.transport {
        UpstreamTransport::Mock => {
            state
                .upstream
                .forward_chat_completion(&HeaderMap::new(), bytes)
                .await
        }
        UpstreamTransport::Http { base_url, api_key } => {
            state
                .http_forwarder
                .forward_dynamic(&base_url, api_key.as_deref(), bytes)
                .await
        }
    };

    match forwarded {
        Ok(response) => response,
        Err(err) => json_error(StatusCode::BAD_GATEWAY, err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_kernels::{LockSpec, NoiseBitmask1024, MAX_TOPK};

    fn sample_lock(epsilon: f32) -> DomainHypercubeLock {
        let lo = AlignedVec1024::zeros();
        let mut hi = AlignedVec1024::zeros();
        hi.data.fill(1.0);
        DomainHypercubeLock::from_spec(LockSpec {
            domain_id: "unit".to_string(),
            centroid: AlignedVec1024::zeros(),
            lo,
            hi,
            epsilon,
            noise_mask: NoiseBitmask1024::new_bge_m3_default(),
            top_k_indices: [0u16; MAX_TOPK],
            top_k_len: 1,
            quorum_k: 103,
            l1_cosine_threshold: 0.65,
            l2_clause_threshold: 0.65,
            polarity: 0,
            domain_version: 1,
        })
    }

    #[test]
    fn overrides_clone_thresholds_and_redilate_epsilon() {
        let source = sample_lock(0.05);
        let updated = apply_overrides(
            &source,
            &ChatOverridesDto {
                l1_threshold: Some(0.5),
                l2_threshold: Some(0.4),
                quorum_k: Some(11),
                epsilon: Some(0.2),
            },
        );
        assert_eq!(source.epsilon.to_bits(), 0.05f32.to_bits());
        assert_eq!(source.l1_cosine_threshold.to_bits(), 0.65f32.to_bits());
        assert_eq!(source.quorum_k, 103);
        assert_eq!(updated.l1_cosine_threshold.to_bits(), 0.5f32.to_bits());
        assert_eq!(updated.l2_clause_threshold.to_bits(), 0.4f32.to_bits());
        assert_eq!(updated.quorum_k, 11);
        assert_eq!(updated.epsilon.to_bits(), 0.2f32.to_bits());
        let raw_lo = source.lo_dilated.data[0] + source.epsilon;
        let raw_hi = source.hi_dilated.data[0] - source.epsilon;
        assert_eq!(
            updated.lo_dilated.data[0].to_bits(),
            (raw_lo - 0.2).to_bits()
        );
        assert_eq!(
            updated.hi_dilated.data[0].to_bits(),
            (raw_hi + 0.2).to_bits()
        );
        assert_eq!(updated.domain_id, source.domain_id);
    }
}
