//! OpenAI-compatible proxy. Slice 1 is the ingress-to-decision skeleton.

mod cascade;
mod embedder;
mod handler;
mod normalize;
mod routing;
mod segmenter;
mod tei;
mod telemetry;
mod upstream;

pub use cascade::{evaluate_three_tier_cascade, sha256_hex};
pub use embedder::{BatchEmbedderClient, MockBatchEmbedder};
pub use handler::{handle_v1_chat_completions, router, ProxyAppState};
pub use normalize::validate_and_normalize_f32_slice;
pub use segmenter::segment_prompt_clauses;
pub use tei::{embed_request, TeiGrpcEmbedder, TeiHttpEmbedder};
pub use telemetry::BreachErrorEnvelope;
pub use upstream::{MockUpstreamForwarder, ReqwestChatForwarder, UpstreamLlmForwarder};
