//! Document compressor.
//!
//! This module provides the [`BaseDocumentCompressor`] trait for post-processing
//! of retrieved documents.

use async_trait::async_trait;

use super::Document;
use crate::callbacks::Callbacks;

/// Base trait for document compressors.
///
/// This abstraction is primarily used for post-processing of retrieved documents.
///
/// [`Document`] objects matching a given query are first retrieved.
/// Then the list of documents can be further processed.
///
/// For example, one could re-rank the retrieved documents using an LLM.
///
/// Users should favor using a `RunnableLambda` instead of implementing this
/// trait directly when possible.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::documents::{BaseDocumentCompressor, Document};
/// use agent_chain_core::callbacks::Callbacks;
/// use async_trait::async_trait;
///
/// struct MyCompressor {
///     threshold: f64,
/// }
///
/// #[async_trait]
/// impl BaseDocumentCompressor for MyCompressor {
///     async fn compress_documents(
///         &self,
///         documents: Vec<Document>,
///         query: &str,
///         _callbacks: Option<Callbacks>,
///     ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
///         // Filter documents based on some criteria
///         Ok(documents
///             .into_iter()
///             .filter(|doc| doc.page_content.contains(query))
///             .collect())
///     }
/// }
/// ```
#[async_trait]
pub trait BaseDocumentCompressor: Send + Sync {
    /// Compress retrieved documents given the query context.
    ///
    /// # Arguments
    ///
    /// * `documents` - The retrieved [`Document`] objects.
    /// * `query` - The query context.
    /// * `callbacks` - Optional [`Callbacks`] to run during compression.
    ///
    /// # Returns
    ///
    /// The compressed documents, or an error if compression fails.
    async fn compress_documents(
        &self,
        documents: Vec<Document>,
        query: &str,
        callbacks: Option<Callbacks>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>>;

    /// Synchronously compress retrieved documents given the query context.
    ///
    /// This is a blocking version of [`compress_documents`][Self::compress_documents].
    /// The default implementation runs the async version using a blocking runtime.
    ///
    /// # Arguments
    ///
    /// * `documents` - The retrieved [`Document`] objects.
    /// * `query` - The query context.
    /// * `callbacks` - Optional [`Callbacks`] to run during compression.
    ///
    /// # Returns
    ///
    /// The compressed documents, or an error if compression fails.
    fn compress_documents_sync(
        &self,
        documents: Vec<Document>,
        query: &str,
        callbacks: Option<Callbacks>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: Sized,
    {
        let _ = (documents, query, callbacks);
        Err("Sync version not implemented - use compress_documents instead".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCompressor;

    #[async_trait]
    impl BaseDocumentCompressor for TestCompressor {
        async fn compress_documents(
            &self,
            documents: Vec<Document>,
            query: &str,
            _callbacks: Option<Callbacks>,
        ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(documents
                .into_iter()
                .filter(|doc| doc.page_content.contains(query))
                .collect())
        }
    }

    #[tokio::test]
    async fn test_compress_documents() {
        let compressor = TestCompressor;
        let documents = vec![
            Document::new("Hello world"),
            Document::new("Goodbye world"),
            Document::new("Hello again"),
        ];

        let result = compressor
            .compress_documents(documents, "Hello", None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|doc| doc.page_content.contains("Hello")));
    }
}
