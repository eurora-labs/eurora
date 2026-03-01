use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use super::Document;

#[async_trait]
pub trait BaseDocumentTransformer: Send + Sync {
    fn transform_documents(
        &self,
        documents: Vec<Document>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>>;

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
                    Document::builder()
                        .page_content(doc.page_content.to_uppercase())
                        .metadata(doc.metadata)
                        .build()
                })
                .collect())
        }
    }

    #[test]
    fn test_transform_documents() {
        let transformer = UppercaseTransformer;
        let documents = vec![
            Document::builder().page_content("hello world").build(),
            Document::builder().page_content("goodbye world").build(),
        ];

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
        let documents = vec![Document::builder().page_content("hello world").build()];

        let result = transformer
            .atransform_documents(documents, HashMap::new())
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page_content, "HELLO WORLD");
    }
}
