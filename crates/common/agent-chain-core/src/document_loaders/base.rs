use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::documents::base::{Blob, Document};
use crate::text_splitters::TextSplitter;

#[async_trait]
pub trait BaseLoader: Send + Sync {
    fn lazy_load(&self) -> Box<dyn Iterator<Item = Document> + '_>;

    fn load(&self) -> Vec<Document> {
        self.lazy_load().collect()
    }

    fn load_and_split(
        &self,
        text_splitter: &dyn TextSplitter,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let docs = self.load();
        text_splitter.split_documents(&docs)
    }

    async fn alazy_load(&self) -> Pin<Box<dyn Stream<Item = Document> + Send + '_>> {
        let docs: Vec<Document> = self.lazy_load().collect();
        Box::pin(futures::stream::iter(docs))
    }

    async fn aload(&self) -> Vec<Document> {
        use futures::StreamExt;
        self.alazy_load().await.collect().await
    }
}

pub trait BaseBlobParser: Send + Sync {
    fn lazy_parse(&self, blob: &Blob) -> Box<dyn Iterator<Item = Document> + '_>;

    fn parse(&self, blob: &Blob) -> Vec<Document> {
        self.lazy_parse(blob).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestLoader {
        docs: Vec<Document>,
    }

    #[async_trait]
    impl BaseLoader for TestLoader {
        fn lazy_load(&self) -> Box<dyn Iterator<Item = Document> + '_> {
            Box::new(self.docs.iter().cloned())
        }
    }

    struct HalfSplitter;

    #[async_trait]
    impl crate::documents::BaseDocumentTransformer for HalfSplitter {
        fn transform_documents(
            &self,
            documents: Vec<Document>,
            _kwargs: HashMap<String, serde_json::Value>,
        ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
            self.split_documents(&documents)
        }
    }

    #[async_trait]
    impl TextSplitter for HalfSplitter {
        fn split_text(
            &self,
            text: &str,
        ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
            let mid = text.len() / 2;
            if mid == 0 {
                return Ok(vec![text.to_string()]);
            }
            Ok(vec![text[..mid].to_string(), text[mid..].to_string()])
        }
    }

    #[test]
    fn test_load() {
        let loader = TestLoader {
            docs: vec![
                Document::builder().page_content("hello").build(),
                Document::builder().page_content("world").build(),
            ],
        };
        let docs = loader.load();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].page_content, "hello");
        assert_eq!(docs[1].page_content, "world");
    }

    #[test]
    fn test_load_and_split() {
        let loader = TestLoader {
            docs: vec![
                Document::builder().page_content("abcdef").build(),
                Document::builder().page_content("1234").build(),
            ],
        };
        let splitter = HalfSplitter;
        let docs = loader.load_and_split(&splitter).unwrap();
        assert_eq!(docs.len(), 4);
        assert_eq!(docs[0].page_content, "abc");
        assert_eq!(docs[1].page_content, "def");
        assert_eq!(docs[2].page_content, "12");
        assert_eq!(docs[3].page_content, "34");
    }

    #[tokio::test]
    async fn test_aload() {
        let loader = TestLoader {
            docs: vec![Document::builder().page_content("async doc").build()],
        };
        let docs = loader.aload().await;
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].page_content, "async doc");
    }

    #[tokio::test]
    async fn test_alazy_load_stream() {
        use futures::StreamExt;

        let loader = TestLoader {
            docs: vec![
                Document::builder().page_content("a").build(),
                Document::builder().page_content("b").build(),
                Document::builder().page_content("c").build(),
            ],
        };
        let mut stream = loader.alazy_load().await;
        let first = stream.next().await;
        assert_eq!(first.unwrap().page_content, "a");
        let rest: Vec<Document> = stream.collect().await;
        assert_eq!(rest.len(), 2);
    }
}
