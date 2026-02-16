//! Abstract interface for document loader implementations.
//!
//! Mirrors `langchain_core.document_loaders.base`.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::documents::base::{Blob, Document};
use crate::text_splitters::TextSplitter;

/// Interface for Document Loader.
///
/// Implementations should implement the lazy-loading method using iterators
/// to avoid loading all documents into memory at once.
///
/// `load` is provided just for user convenience and should not be overridden.
#[async_trait]
pub trait BaseLoader: Send + Sync {
    /// A lazy loader for [`Document`].
    ///
    /// Subclasses should implement this method to define how documents are loaded.
    fn lazy_load(&self) -> Box<dyn Iterator<Item = Document> + '_>;

    /// Load data into [`Document`] objects.
    fn load(&self) -> Vec<Document> {
        self.lazy_load().collect()
    }

    /// Load documents and split into chunks using a text splitter.
    fn load_and_split(
        &self,
        text_splitter: &dyn TextSplitter,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let docs = self.load();
        text_splitter.split_documents(&docs)
    }

    /// A lazy async loader for [`Document`].
    ///
    /// Default implementation eagerly collects `lazy_load` and wraps the
    /// result in a stream. Concrete async loaders should override this
    /// to provide truly lazy async loading.
    async fn alazy_load(&self) -> Pin<Box<dyn Stream<Item = Document> + Send + '_>> {
        let docs: Vec<Document> = self.lazy_load().collect();
        Box::pin(futures::stream::iter(docs))
    }

    /// Async load data into [`Document`] objects.
    async fn aload(&self) -> Vec<Document> {
        use futures::StreamExt;
        self.alazy_load().await.collect().await
    }
}

/// Abstract interface for blob parsers.
///
/// A blob parser provides a way to parse raw data stored in a blob into one
/// or more [`Document`] objects.
///
/// The parser can be composed with blob loaders, making it easy to reuse
/// a parser independent of how the blob was originally loaded.
pub trait BaseBlobParser: Send + Sync {
    /// Lazy parsing interface.
    ///
    /// Subclasses are required to implement this method.
    fn lazy_parse(&self, blob: &Blob) -> Box<dyn Iterator<Item = Document> + '_>;

    /// Eagerly parse the blob into a list of [`Document`] objects.
    ///
    /// This is a convenience method for interactive development.
    /// Production applications should favor the `lazy_parse` method instead.
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
            docs: vec![Document::new("hello"), Document::new("world")],
        };
        let docs = loader.load();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].page_content, "hello");
        assert_eq!(docs[1].page_content, "world");
    }

    #[test]
    fn test_load_and_split() {
        let loader = TestLoader {
            docs: vec![Document::new("abcdef"), Document::new("1234")],
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
            docs: vec![Document::new("async doc")],
        };
        let docs = loader.aload().await;
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].page_content, "async doc");
    }

    #[tokio::test]
    async fn test_alazy_load_stream() {
        use futures::StreamExt;

        let loader = TestLoader {
            docs: vec![Document::new("a"), Document::new("b"), Document::new("c")],
        };
        let mut stream = loader.alazy_load().await;
        let first = stream.next().await;
        assert_eq!(first.unwrap().page_content, "a");
        let rest: Vec<Document> = stream.collect().await;
        assert_eq!(rest.len(), 2);
    }
}
