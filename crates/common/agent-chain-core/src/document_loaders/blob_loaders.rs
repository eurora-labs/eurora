//! Schema for Blobs and Blob Loaders.
//!
//! The goal is to facilitate decoupling of content loading from content parsing code.
//!
//! In addition, content loading code should provide a lazy loading interface by default.

use std::path::PathBuf;

pub use crate::documents::base::Blob;

/// A path-like type, equivalent to Python's `str | PurePath`.
pub type PathLike = PathBuf;

/// Abstract interface for blob loaders implementation.
///
/// Implementers should be able to load raw content from a storage system according
/// to some criteria and return the raw content lazily as a stream of blobs.
pub trait BlobLoader: Send + Sync {
    /// A lazy loader for raw data represented by [`Blob`] objects.
    fn yield_blobs(&self) -> Box<dyn Iterator<Item = Blob> + '_>;
}
