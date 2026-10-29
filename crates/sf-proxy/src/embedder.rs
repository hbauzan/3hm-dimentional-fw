//! Embedder seam. Slice 1 ships the mock. TEI adapters land in Slice 4.

use async_trait::async_trait;
use sf_kernels::AlignedVec1024;

#[async_trait]
pub trait BatchEmbedderClient: Send + Sync {
    async fn embed_batch_f32(&self, texts: &[&str]) -> Result<Vec<AlignedVec1024>, String>;
}

/// Maps the exact token `BREACH` to `breach`, every other text to `pass_vector`.
pub struct MockBatchEmbedder {
    pub pass_vector: AlignedVec1024,
    pub breach_vector: AlignedVec1024,
}

#[async_trait]
impl BatchEmbedderClient for MockBatchEmbedder {
    async fn embed_batch_f32(&self, texts: &[&str]) -> Result<Vec<AlignedVec1024>, String> {
        Ok(texts
            .iter()
            .map(|text| {
                if *text == "BREACH" {
                    self.breach_vector
                } else {
                    self.pass_vector
                }
            })
            .collect())
    }
}
