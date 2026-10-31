//! OpenAI-compatible proxy and gateway daemon.

mod cascade;
mod chat_api;
mod cors;
mod embedder;
mod handler;
mod normalize;
mod routing;
mod segmenter;
mod tei;
mod telemetry;
mod upstream;

pub use cascade::{evaluate_three_tier_cascade, sha256_hex};
pub use chat_api::{
    apply_chat_overrides, chat_upstream_body, evaluate_chat_cascade, latest_chat_user_text,
    resolve_chat_upstream, ChatBreachDto, ChatMessageDto, ChatOverridesDto, ChatReject,
    ChatRequestDto, ChatUpstream, Tier1BreachDto, Tier2BreachDto, Tier3BreachDto,
    UpstreamConfigDto,
};
pub use cors::CorsAllow;
pub use embedder::{BatchEmbedderClient, MockBatchEmbedder};
pub use handler::{handle_v1_chat_completions, router, router_with_cors, ProxyAppState};
pub use normalize::validate_and_normalize_f32_slice;
pub use segmenter::segment_prompt_clauses;
pub use tei::{embed_request, TeiGrpcEmbedder, TeiHttpEmbedder};
pub use telemetry::BreachErrorEnvelope;
pub use upstream::{MockUpstreamForwarder, ReqwestChatForwarder, UpstreamLlmForwarder};
