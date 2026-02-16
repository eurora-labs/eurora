use async_trait::async_trait;

use crate::Result;

/// Interface for embedding models.
///
/// Text embedding models are used to map text to a vector (a point in n-dimensional
/// space). Texts that are similar will usually be mapped to points that are close to
/// each other in this space.
///
/// This abstraction contains a method for embedding a list of documents and a method
/// for embedding a query text. The embedding of a query text is expected to be a single
/// vector, while the embedding of a list of documents is expected to be a list of
/// vectors.
///
/// Usually the query embedding is identical to the document embedding, but the
/// abstraction allows treating them independently.
#[async_trait]
pub trait Embeddings: Send + Sync {
    /// Embed search docs.
    fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    /// Embed query text.
    fn embed_query(&self, text: &str) -> Result<Vec<f32>>;

    /// Asynchronous embed search docs.
    ///
    /// By default delegates to the synchronous implementation.
    async fn aembed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed_documents(texts)
    }

    /// Asynchronous embed query text.
    ///
    /// By default delegates to the synchronous implementation.
    async fn aembed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_query(text)
    }
}
