use async_trait::async_trait;

use super::Document;
use crate::callbacks::Callbacks;

#[async_trait]
pub trait BaseDocumentCompressor: Send + Sync {
    async fn compress_documents(
        &self,
        documents: Vec<Document>,
        query: &str,
        callbacks: Option<Callbacks>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>>;

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
            Document::builder().page_content("Hello world").build(),
            Document::builder().page_content("Goodbye world").build(),
            Document::builder().page_content("Hello again").build(),
        ];

        let result = compressor
            .compress_documents(documents, "Hello", None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|doc| doc.page_content.contains("Hello")));
    }
}
