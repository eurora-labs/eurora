use std::path::PathBuf;

pub use crate::documents::base::Blob;

pub type PathLike = PathBuf;

pub trait BlobLoader: Send + Sync {
    fn yield_blobs(&self) -> Box<dyn Iterator<Item = Blob> + '_>;
}
