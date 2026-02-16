//! Document transformers.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use super::Document;

/// Abstract base trait for document transformation.
///
/// A document transformation takes a sequence of Documents and returns a
/// sequence of transformed Documents.
#[async_trait]
pub trait BaseDocumentTransformer: Send + Sync {
    /// Transform a list of documents.
    fn transform_documents(
        &self,
        documents: Vec<Document>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>>;

    /// Asynchronously transform a list of documents.
    ///
    /// Default implementation delegates to the sync version.
    async fn atransform_documents(
        &self,
        documents: Vec<Document>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        self.transform_documents(documents, kwargs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UppercaseTransformer;

    #[async_trait]
    impl BaseDocumentTransformer for UppercaseTransformer {
        fn transform_documents(
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

    #[test]
    fn test_transform_documents() {
        let transformer = UppercaseTransformer;
        let documents = vec![Document::new("hello world"), Document::new("goodbye world")];

        let result = transformer
            .transform_documents(documents, HashMap::new())
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_content, "HELLO WORLD");
        assert_eq!(result[1].page_content, "GOODBYE WORLD");
    }

    #[tokio::test]
    async fn test_atransform_documents() {
        let transformer = UppercaseTransformer;
        let documents = vec![Document::new("hello world")];

        let result = transformer
            .atransform_documents(documents, HashMap::new())
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page_content, "HELLO WORLD");
    }
}
