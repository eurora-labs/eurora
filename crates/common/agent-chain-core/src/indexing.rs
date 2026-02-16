pub mod api;
pub mod base;
pub mod in_memory;

pub use api::{
    CleanupMode, HashAlgorithm, IndexConfig, IndexDestination, IndexingResult, KeyEncoder,
    NAMESPACE_UUID, SourceIdKey, aindex, get_document_with_hash, index,
};
pub use base::{
    DeleteResponse, DocumentIndex, InMemoryRecordManager, RecordManager, UpsertResponse,
};
pub use in_memory::InMemoryDocumentIndex;
