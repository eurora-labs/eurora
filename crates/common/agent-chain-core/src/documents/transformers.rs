use async_trait::async_trait;

use super::Document;

#[async_trait]
pub trait BaseDocumentTransformer: Send + Sync {
    fn transform_documents(&self, documents: &[Document]) -> crate::error::Result<Vec<Document>>;

    async fn transform_documents_async(
        &self,
        documents: &[Document],
    ) -> crate::error::Result<Vec<Document>> {
        self.transform_documents(documents)
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
            documents: &[Document],
        ) -> crate::error::Result<Vec<Document>> {
            Ok(documents
                .iter()
                .map(|doc| {
                    Document::builder()
                        .page_content(doc.page_content().to_uppercase())
                        .metadata(doc.metadata().clone())
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

        let result = transformer.transform_documents(&documents).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_content(), "HELLO WORLD");
        assert_eq!(result[1].page_content(), "GOODBYE WORLD");
    }

    #[tokio::test]
    async fn test_transform_documents_async() {
        let transformer = UppercaseTransformer;
        let documents = vec![Document::builder().page_content("hello world").build()];

        let result = transformer
            .transform_documents_async(&documents)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page_content(), "HELLO WORLD");
    }
}
