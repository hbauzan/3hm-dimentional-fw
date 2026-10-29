//! OpenAI-compatible proxy. Slice 1 is the ingress-to-decision skeleton.

mod cascade;
mod embedder;
mod handler;
mod segmenter;
mod telemetry;
mod upstream;

pub use cascade::{evaluate_three_tier_cascade, sha256_hex};
pub use embedder::{BatchEmbedderClient, MockBatchEmbedder};
pub use handler::{handle_v1_chat_completions, router, ProxyAppState};
pub use segmenter::segment_prompt_clauses;
pub use telemetry::BreachErrorEnvelope;
pub use upstream::{MockUpstreamForwarder, UpstreamLlmForwarder};
