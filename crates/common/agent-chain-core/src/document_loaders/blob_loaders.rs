pub trait BlobLoader: Send + Sync {
    fn blobs(&self) -> Box<dyn Iterator<Item = crate::documents::base::Blob> + '_>;
}
