//! Abstract interface for document loader implementations.

use async_trait::async_trait;

use crate::documents::base::{Blob, Document};

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

    /// A lazy async loader for [`Document`].
    ///
    /// Default implementation wraps `lazy_load` via `spawn_blocking`.
    async fn alazy_load(&self) -> Box<dyn Iterator<Item = Document> + Send> {
        let docs: Vec<Document> = self.lazy_load().collect();
        Box::new(docs.into_iter())
    }

    /// Async load data into [`Document`] objects.
    async fn aload(&self) -> Vec<Document> {
        self.alazy_load().await.collect()
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
