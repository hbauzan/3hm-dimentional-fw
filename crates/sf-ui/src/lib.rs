//! Three-Headed Semantic Firewall Diagnostic Studio crate (`sf-ui`).
//!
//! Provides the embedded Web Diagnostic Studio, REST APIs, chat cascade, and
//! full-1024D coordinate inspector.

pub mod chat;
mod embedder;
pub mod handlers;
pub mod inspector;

pub use handlers::*;
pub use inspector::*;
