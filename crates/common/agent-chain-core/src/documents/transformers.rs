//! Document transformers.
//!
//! This module provides the [`BaseDocumentTransformer`] trait for document
//! transformation operations.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use super::Document;

/// Abstract base trait for document transformation.
///
/// A document transformation takes a sequence of [`Document`] objects and returns a
/// sequence of transformed [`Document`] objects.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::documents::{BaseDocumentTransformer, Document};
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use serde_json::Value;
///
/// struct EmbeddingsRedundantFilter {
///     similarity_threshold: f64,
/// }
///
/// #[async_trait]
/// impl BaseDocumentTransformer for EmbeddingsRedundantFilter {
///     async fn transform_documents(
///         &self,
///         documents: Vec<Document>,
///         _kwargs: HashMap<String, Value>,
///     ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
///         // Filter out redundant documents based on embeddings similarity
///         // This is a simplified example
///         Ok(documents)
///     }
/// }
/// ```
#[async_trait]
pub trait BaseDocumentTransformer: Send + Sync {
    /// Transform a list of documents.
    ///
    /// # Arguments
    ///
    /// * `documents` - A sequence of [`Document`] objects to be transformed.
    /// * `kwargs` - Additional keyword arguments for transformation.
    ///
    /// # Returns
    ///
    /// A sequence of transformed [`Document`] objects, or an error if transformation fails.
    async fn transform_documents(
        &self,
        documents: Vec<Document>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>>;

    /// Synchronously transform a list of documents.
    ///
    /// This is a blocking version of [`transform_documents`][Self::transform_documents].
    /// The default implementation indicates that the sync version needs to be implemented
    /// or the async version should be used.
    ///
    /// # Arguments
    ///
    /// * `documents` - A sequence of [`Document`] objects to be transformed.
    /// * `kwargs` - Additional keyword arguments for transformation.
    ///
    /// # Returns
    ///
    /// A sequence of transformed [`Document`] objects, or an error if transformation fails.
    fn transform_documents_sync(
        &self,
        documents: Vec<Document>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: Sized,
    {
        // Note: In a real implementation, this would need to be handled differently
        // as we can't easily call async from sync without a runtime.
        // This is a placeholder that indicates the sync version needs to be implemented.
        let _ = (documents, kwargs);
        Err("Sync version not implemented - use transform_documents instead".into())
    }
}

/// A simple document transformer that applies a function to each document.
pub struct FunctionTransformer<F>
where
    F: Fn(Document) -> Document + Send + Sync,
{
    transform_fn: F,
}

impl<F> FunctionTransformer<F>
where
    F: Fn(Document) -> Document + Send + Sync,
{
    /// Create a new FunctionTransformer with the given function.
    pub fn new(transform_fn: F) -> Self {
        Self { transform_fn }
    }
}

#[async_trait]
impl<F> BaseDocumentTransformer for FunctionTransformer<F>
where
    F: Fn(Document) -> Document + Send + Sync,
{
    async fn transform_documents(
        &self,
        documents: Vec<Document>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(documents.into_iter().map(&self.transform_fn).collect())
    }

    fn transform_documents_sync(
        &self,
        documents: Vec<Document>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(documents.into_iter().map(&self.transform_fn).collect())
    }
}

/// A document transformer that filters documents based on a predicate.
pub struct FilterTransformer<F>
where
    F: Fn(&Document) -> bool + Send + Sync,
{
    filter_fn: F,
}

impl<F> FilterTransformer<F>
where
    F: Fn(&Document) -> bool + Send + Sync,
{
    /// Create a new FilterTransformer with the given predicate.
    pub fn new(filter_fn: F) -> Self {
        Self { filter_fn }
    }
}

#[async_trait]
impl<F> BaseDocumentTransformer for FilterTransformer<F>
where
    F: Fn(&Document) -> bool + Send + Sync,
{
    async fn transform_documents(
        &self,
        documents: Vec<Document>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(documents.into_iter().filter(&self.filter_fn).collect())
    }

    fn transform_documents_sync(
        &self,
        documents: Vec<Document>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(documents.into_iter().filter(&self.filter_fn).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UppercaseTransformer;

    #[async_trait]
    impl BaseDocumentTransformer for UppercaseTransformer {
        async fn transform_documents(
            &self,
            documents: Vec<Document>,
            _kwargs: HashMap<String, Value>,
        ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(documents
                .into_iter()
                .map(|doc| {
                    Document::new(doc.page_content.to_uppercase()).with_metadata(doc.metadata)
                })
                .collect())
        }
    }

    #[tokio::test]
    async fn test_transform_documents() {
        let transformer = UppercaseTransformer;
        let documents = vec![Document::new("hello world"), Document::new("goodbye world")];

        let result = transformer
            .transform_documents(documents, HashMap::new())
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_content, "HELLO WORLD");
        assert_eq!(result[1].page_content, "GOODBYE WORLD");
    }

    #[tokio::test]
    async fn test_function_transformer() {
        let transformer = FunctionTransformer::new(|doc| {
            Document::new(format!("[PROCESSED] {}", doc.page_content))
        });

        let documents = vec![Document::new("test")];

        let result = transformer
            .transform_documents(documents, HashMap::new())
            .await
            .unwrap();

        assert_eq!(result[0].page_content, "[PROCESSED] test");
    }

    #[tokio::test]
    async fn test_filter_transformer() {
        let transformer = FilterTransformer::new(|doc| doc.page_content.len() > 5);

        let documents = vec![
            Document::new("hi"),
            Document::new("hello world"),
            Document::new("bye"),
        ];

        let result = transformer
            .transform_documents(documents, HashMap::new())
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page_content, "hello world");
    }
}
