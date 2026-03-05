use std::pin::Pin;

use futures::Stream;

use crate::documents::BaseDocumentTransformer;
use crate::documents::base::Document;

pub trait BaseLoader: Send + Sync {
    fn lazy_load(&self) -> Box<dyn Iterator<Item = Document> + '_>;

    fn load(&self) -> Vec<Document> {
        self.lazy_load().collect()
    }

    fn load_and_split(
        &self,
        text_splitter: &dyn BaseDocumentTransformer,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let docs = self.load();
        text_splitter
            .transform_documents(&docs)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Returns an async stream of documents.
    ///
    /// The default implementation eagerly collects from `lazy_load` then wraps
    /// the result in a stream. Override this for truly lazy async iteration.
    fn alazy_load(
        &self,
    ) -> impl std::future::Future<Output = Pin<Box<dyn Stream<Item = Document> + Send + '_>>> + Send
    {
        async {
            Box::pin(futures::stream::iter(self.load()))
                as Pin<Box<dyn Stream<Item = Document> + Send + '_>>
        }
    }

    fn aload(&self) -> impl std::future::Future<Output = Vec<Document>> + Send {
        async {
            use futures::StreamExt;
            self.alazy_load().await.collect().await
        }
    }
}

pub trait BaseBlobParser: Send + Sync {
    fn lazy_parse(
        &self,
        blob: &crate::documents::base::Blob,
    ) -> Box<dyn Iterator<Item = Document> + '_>;

    fn parse(&self, blob: &crate::documents::base::Blob) -> Vec<Document> {
        self.lazy_parse(blob).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestLoader {
        docs: Vec<Document>,
    }

    impl BaseLoader for TestLoader {
        fn lazy_load(&self) -> Box<dyn Iterator<Item = Document> + '_> {
            Box::new(self.docs.iter().cloned())
        }
    }

    struct HalfSplitter;

    impl HalfSplitter {
        fn new() -> Self {
            Self
        }
    }

    impl crate::documents::BaseDocumentTransformer for HalfSplitter {
        fn transform_documents(
            &self,
            documents: &[Document],
        ) -> crate::error::Result<Vec<Document>> {
            let mut result = Vec::new();
            for doc in documents {
                let text = doc.page_content();
                let mid = text.len() / 2;
                let mid = text.floor_char_boundary(mid);
                if mid == 0 {
                    result.push(doc.clone());
                } else {
                    result.push(Document::builder().page_content(&text[..mid]).build());
                    result.push(Document::builder().page_content(&text[mid..]).build());
                }
            }
            Ok(result)
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
        assert_eq!(docs[0].page_content(), "hello");
        assert_eq!(docs[1].page_content(), "world");
    }

    #[test]
    fn test_load_and_split() {
        let loader = TestLoader {
            docs: vec![
                Document::builder().page_content("abcdef").build(),
                Document::builder().page_content("1234").build(),
            ],
        };
        let splitter = HalfSplitter::new();
        let docs = loader.load_and_split(&splitter).unwrap();
        assert_eq!(docs.len(), 4);
        assert_eq!(docs[0].page_content(), "abc");
        assert_eq!(docs[1].page_content(), "def");
        assert_eq!(docs[2].page_content(), "12");
        assert_eq!(docs[3].page_content(), "34");
    }

    #[tokio::test]
    async fn test_aload() {
        let loader = TestLoader {
            docs: vec![Document::builder().page_content("async doc").build()],
        };
        let docs = loader.aload().await;
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].page_content(), "async doc");
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
        assert_eq!(first.unwrap().page_content(), "a");
        let rest: Vec<Document> = stream.collect().await;
        assert_eq!(rest.len(), 2);
    }
}
