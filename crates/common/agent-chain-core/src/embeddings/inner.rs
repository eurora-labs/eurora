use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait Embeddings: Send + Sync {
    async fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>>;
}
