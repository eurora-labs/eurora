use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait Embeddings: Send + Sync {
    fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    fn embed_query(&self, text: &str) -> Result<Vec<f32>>;

    async fn aembed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed_documents(texts)
    }

    async fn aembed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_query(text)
    }
}
