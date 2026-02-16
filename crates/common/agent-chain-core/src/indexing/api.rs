use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use blake2::Blake2b512;
use serde::Serialize;
use serde_json::Value;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use uuid::Uuid;

use crate::documents::Document;
use crate::error::{Error, Result};
use crate::indexing::base::{DocumentIndex, RecordManager};
use crate::vectorstores::VectorStore;

/// Magic UUID namespace for deterministic hashing.
/// Equivalent to Python's `uuid.UUID(int=1984)`.
pub const NAMESPACE_UUID: Uuid = Uuid::from_u128(1984);

static WARNED_ABOUT_SHA1: AtomicBool = AtomicBool::new(false);

/// Hash algorithm selection for document hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Sha1,
    Sha256,
    Sha512,
    Blake2b,
}

/// Cleanup mode for the indexing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupMode {
    Incremental,
    Full,
    ScopedFull,
}

/// How to encode document keys for hashing.
pub enum KeyEncoder {
    Algorithm(HashAlgorithm),
    Custom(Box<dyn Fn(&Document) -> String + Send + Sync>),
}

/// How to extract source IDs from documents.
pub enum SourceIdKey {
    MetadataKey(String),
    Custom(Box<dyn Fn(&Document) -> Option<String> + Send + Sync>),
}

/// Where to index documents.
pub enum IndexDestination<'a> {
    VectorStore(&'a dyn VectorStore),
    DocumentIndex(&'a dyn DocumentIndex),
}

/// Configuration for the `index` / `aindex` functions.
pub struct IndexConfig {
    pub batch_size: usize,
    pub cleanup: Option<CleanupMode>,
    pub source_id_key: Option<SourceIdKey>,
    pub cleanup_batch_size: usize,
    pub force_update: bool,
    pub key_encoder: KeyEncoder,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            cleanup: None,
            source_id_key: None,
            cleanup_batch_size: 1_000,
            force_update: false,
            key_encoder: KeyEncoder::Algorithm(HashAlgorithm::Sha1),
        }
    }
}

/// Result of an indexing operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexingResult {
    pub num_added: usize,
    pub num_updated: usize,
    pub num_deleted: usize,
    pub num_skipped: usize,
}

fn warn_about_sha1() {
    if !WARNED_ABOUT_SHA1.swap(true, Ordering::Relaxed) {
        tracing::warn!(
            "Using SHA-1 for document hashing. SHA-1 is *not* collision-resistant; \
             a motivated attacker can construct distinct inputs that map to the same \
             fingerprint. Switch to a stronger algorithm such as 'blake2b', 'sha256', \
             or 'sha512' by specifying the `key_encoder` parameter."
        );
    }
}

/// Return a hex digest of `text` using the specified algorithm.
///
/// Note: SHA-1 path is special -- it wraps the hex digest in uuid5(NAMESPACE, hex).
/// Other algorithms return the raw hex digest.
fn calculate_hash(text: &str, algorithm: HashAlgorithm) -> String {
    use sha1::Digest as _;

    match algorithm {
        HashAlgorithm::Sha1 => {
            let digest = Sha1::digest(text.as_bytes());
            let hex = format!("{:x}", digest);
            Uuid::new_v5(&NAMESPACE_UUID, hex.as_bytes()).to_string()
        }
        HashAlgorithm::Sha256 => {
            let digest = Sha256::digest(text.as_bytes());
            format!("{:x}", digest)
        }
        HashAlgorithm::Sha512 => {
            let digest = Sha512::digest(text.as_bytes());
            format!("{:x}", digest)
        }
        HashAlgorithm::Blake2b => {
            let digest = Blake2b512::digest(text.as_bytes());
            format!("{:x}", digest)
        }
    }
}

/// A JSON formatter that matches Python's json.dumps default format:
/// `", "` between items and `": "` between key-value pairs.
struct PythonJsonFormatter;

impl serde_json::ser::Formatter for PythonJsonFormatter {
    fn begin_array_value<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_key<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_value<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        writer.write_all(b": ")
    }
}

/// Serialize metadata with sorted keys for deterministic hashing.
fn sorted_json_string(metadata: &std::collections::HashMap<String, Value>) -> Result<String> {
    let sorted: BTreeMap<&String, &Value> = metadata.iter().collect();
    // Python's json.dumps uses ", " and ": " as separators by default.
    // serde_json::to_string uses "," and ":" (no spaces).
    // We must match Python's format for hash compatibility.
    let mut buf = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, PythonJsonFormatter);
    sorted.serialize(&mut serializer).map_err(|e| {
        Error::Other(format!(
            "Failed to hash metadata: {e}. Please use a dict that can be serialized using json."
        ))
    })?;
    String::from_utf8(buf).map_err(|e| Error::Other(format!("Invalid UTF-8 in JSON: {e}")))
}

/// Calculate a hash of the document and return a new Document with the hash as its ID.
pub fn get_document_with_hash(document: &Document, key_encoder: &KeyEncoder) -> Result<Document> {
    let hash = match key_encoder {
        KeyEncoder::Custom(encoder) => encoder(document),
        KeyEncoder::Algorithm(algorithm) => {
            let content_hash = calculate_hash(&document.page_content, *algorithm);
            let serialized_meta = sorted_json_string(&document.metadata)?;
            let metadata_hash = calculate_hash(&serialized_meta, *algorithm);
            calculate_hash(&format!("{content_hash}{metadata_hash}"), *algorithm)
        }
    };

    Ok(Document {
        id: Some(hash),
        page_content: document.page_content.clone(),
        metadata: document.metadata.clone(),
        type_: document.type_.clone(),
    })
}

/// Deduplicate documents by ID, preserving order.
fn deduplicate_in_order(documents: Vec<Document>) -> Vec<Document> {
    let mut seen = HashSet::new();
    documents
        .into_iter()
        .filter(|doc| {
            if let Some(id) = &doc.id {
                seen.insert(id.clone())
            } else {
                true
            }
        })
        .collect()
}

/// Build a closure that extracts a source ID from a Document.
fn get_source_id_assigner<'a>(
    source_id_key: &'a Option<SourceIdKey>,
) -> Box<dyn Fn(&Document) -> Option<String> + 'a> {
    match source_id_key {
        None => Box::new(|_| None),
        Some(SourceIdKey::MetadataKey(key)) => Box::new(move |doc: &Document| {
            doc.metadata.get(key).and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Null => None,
                other => Some(other.to_string()),
            })
        }),
        Some(SourceIdKey::Custom(f)) => Box::new(move |doc: &Document| f(doc)),
    }
}

/// Batch an iterator into chunks of the given size.
fn batch_iter<T>(size: usize, items: impl IntoIterator<Item = T>) -> Vec<Vec<T>> {
    let mut result = Vec::new();
    let mut current = Vec::with_capacity(size);
    for item in items {
        current.push(item);
        if current.len() >= size {
            result.push(std::mem::take(&mut current));
            current = Vec::with_capacity(size);
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

/// Delete documents from the destination.
fn delete_from_destination(destination: &IndexDestination<'_>, ids: &[String]) -> Result<()> {
    match destination {
        IndexDestination::VectorStore(vs) => {
            vs.delete(Some(ids.to_vec()))?;
            Ok(())
        }
        IndexDestination::DocumentIndex(di) => {
            let response = di.delete(Some(ids))?;
            if let Some(num_failed) = response.num_failed
                && num_failed > 0
            {
                return Err(Error::Indexing(
                    "The delete operation to DocumentIndex failed.".to_string(),
                ));
            }
            Ok(())
        }
    }
}

/// Async delete documents from the destination.
async fn adelete_from_destination(
    destination: &IndexDestination<'_>,
    ids: &[String],
) -> Result<()> {
    match destination {
        IndexDestination::VectorStore(vs) => {
            vs.adelete(Some(ids.to_vec())).await?;
            Ok(())
        }
        IndexDestination::DocumentIndex(di) => {
            let response = di.adelete(Some(ids)).await?;
            if let Some(num_failed) = response.num_failed
                && num_failed > 0
            {
                return Err(Error::Indexing(
                    "The delete operation to DocumentIndex failed.".to_string(),
                ));
            }
            Ok(())
        }
    }
}

/// Add documents to the destination.
fn add_to_destination(
    destination: &IndexDestination<'_>,
    documents: Vec<Document>,
    ids: Vec<String>,
) -> Result<()> {
    match destination {
        IndexDestination::VectorStore(vs) => {
            vs.add_documents(documents, Some(ids))?;
            Ok(())
        }
        IndexDestination::DocumentIndex(di) => {
            di.upsert(&documents)?;
            Ok(())
        }
    }
}

/// Async add documents to the destination.
async fn aadd_to_destination(
    destination: &IndexDestination<'_>,
    documents: Vec<Document>,
    ids: Vec<String>,
) -> Result<()> {
    match destination {
        IndexDestination::VectorStore(vs) => {
            vs.aadd_documents(documents, Some(ids)).await?;
            Ok(())
        }
        IndexDestination::DocumentIndex(di) => {
            di.aupsert(&documents).await?;
            Ok(())
        }
    }
}

/// Index data into a vector store or document index.
///
/// Mirrors `langchain_core.indexing.api.index`.
pub fn index(
    docs_source: impl IntoIterator<Item = Document>,
    record_manager: &dyn RecordManager,
    destination: &IndexDestination<'_>,
    config: &IndexConfig,
) -> Result<IndexingResult> {
    if matches!(
        &config.key_encoder,
        KeyEncoder::Algorithm(HashAlgorithm::Sha1)
    ) {
        warn_about_sha1();
    }

    if matches!(
        config.cleanup,
        Some(CleanupMode::Incremental) | Some(CleanupMode::ScopedFull)
    ) && config.source_id_key.is_none()
    {
        return Err(Error::InvalidConfig(
            "Source id key is required when cleanup mode is incremental or scoped_full."
                .to_string(),
        ));
    }

    let source_id_assigner = get_source_id_assigner(&config.source_id_key);
    let index_start_dt = record_manager.get_time()?;

    let mut num_added: usize = 0;
    let mut num_skipped: usize = 0;
    let mut num_updated: usize = 0;
    let mut num_deleted: usize = 0;
    let mut scoped_full_cleanup_source_ids: HashSet<String> = HashSet::new();

    let doc_batches = batch_iter(config.batch_size, docs_source);

    for doc_batch in doc_batches {
        let original_batch_size = doc_batch.len();

        let hashed_docs: Vec<Document> = doc_batch
            .iter()
            .map(|doc| get_document_with_hash(doc, &config.key_encoder))
            .collect::<Result<Vec<_>>>()?;
        let hashed_docs = deduplicate_in_order(hashed_docs);

        num_skipped += original_batch_size - hashed_docs.len();

        let source_ids: Vec<Option<String>> = hashed_docs.iter().map(&source_id_assigner).collect();

        if matches!(
            config.cleanup,
            Some(CleanupMode::Incremental) | Some(CleanupMode::ScopedFull)
        ) {
            for (source_id, hashed_doc) in source_ids.iter().zip(&hashed_docs) {
                if source_id.is_none() {
                    return Err(Error::InvalidConfig(format!(
                        "Source IDs are required when cleanup mode is incremental or scoped_full. \
                         Document that starts with content: {} was not assigned as source id.",
                        &hashed_doc.page_content[..hashed_doc.page_content.len().min(100)]
                    )));
                }
                if config.cleanup == Some(CleanupMode::ScopedFull) {
                    scoped_full_cleanup_source_ids.insert(source_id.clone().unwrap_or_default());
                }
            }
        }

        let doc_ids: Vec<String> = hashed_docs
            .iter()
            .map(|doc| {
                doc.id
                    .clone()
                    .ok_or_else(|| Error::Indexing("hash should have set document id".to_string()))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let exists_batch = record_manager.exists(&doc_ids)?;

        let mut uids = Vec::new();
        let mut docs_to_index = Vec::new();
        let mut uids_to_refresh = Vec::new();
        let mut seen_docs: HashSet<String> = HashSet::new();

        for (hashed_doc, doc_exists) in hashed_docs.into_iter().zip(exists_batch) {
            let hashed_id = hashed_doc
                .id
                .clone()
                .ok_or_else(|| Error::Indexing("hash should have set document id".to_string()))?;
            if doc_exists {
                if config.force_update {
                    seen_docs.insert(hashed_id.clone());
                } else {
                    uids_to_refresh.push(hashed_id);
                    continue;
                }
            }
            uids.push(hashed_id);
            docs_to_index.push(hashed_doc);
        }

        if !uids_to_refresh.is_empty() {
            record_manager.update(&uids_to_refresh, None, Some(index_start_dt))?;
            num_skipped += uids_to_refresh.len();
        }

        if !docs_to_index.is_empty() {
            add_to_destination(destination, docs_to_index.clone(), uids.clone())?;
            num_added += docs_to_index.len() - seen_docs.len();
            num_updated += seen_docs.len();
        }

        let group_ids_for_update: Vec<Option<String>> = source_ids;
        record_manager.update(&doc_ids, Some(&group_ids_for_update), Some(index_start_dt))?;

        if config.cleanup == Some(CleanupMode::Incremental) {
            let source_ids_for_cleanup: Vec<String> = group_ids_for_update
                .iter()
                .filter_map(|s| s.clone())
                .collect();

            loop {
                let uids_to_delete = record_manager.list_keys(
                    Some(index_start_dt),
                    None,
                    Some(&source_ids_for_cleanup),
                    Some(config.cleanup_batch_size),
                )?;
                if uids_to_delete.is_empty() {
                    break;
                }
                delete_from_destination(destination, &uids_to_delete)?;
                record_manager.delete_keys(&uids_to_delete)?;
                num_deleted += uids_to_delete.len();
            }
        }
    }

    if config.cleanup == Some(CleanupMode::Full)
        || (config.cleanup == Some(CleanupMode::ScopedFull)
            && !scoped_full_cleanup_source_ids.is_empty())
    {
        let delete_group_ids: Option<Vec<String>> =
            if config.cleanup == Some(CleanupMode::ScopedFull) {
                Some(scoped_full_cleanup_source_ids.into_iter().collect())
            } else {
                None
            };

        loop {
            let uids_to_delete = record_manager.list_keys(
                Some(index_start_dt),
                None,
                delete_group_ids.as_deref(),
                Some(config.cleanup_batch_size),
            )?;
            if uids_to_delete.is_empty() {
                break;
            }
            delete_from_destination(destination, &uids_to_delete)?;
            record_manager.delete_keys(&uids_to_delete)?;
            num_deleted += uids_to_delete.len();
        }
    }

    Ok(IndexingResult {
        num_added,
        num_updated,
        num_skipped,
        num_deleted,
    })
}

/// Async index data into a vector store or document index.
///
/// Mirrors `langchain_core.indexing.api.aindex`.
pub async fn aindex(
    docs_source: impl IntoIterator<Item = Document> + Send,
    record_manager: &dyn RecordManager,
    destination: &IndexDestination<'_>,
    config: &IndexConfig,
) -> Result<IndexingResult> {
    if matches!(
        &config.key_encoder,
        KeyEncoder::Algorithm(HashAlgorithm::Sha1)
    ) {
        warn_about_sha1();
    }

    if matches!(
        config.cleanup,
        Some(CleanupMode::Incremental) | Some(CleanupMode::ScopedFull)
    ) && config.source_id_key.is_none()
    {
        return Err(Error::InvalidConfig(
            "Source id key is required when cleanup mode is incremental or scoped_full."
                .to_string(),
        ));
    }

    let source_id_assigner = get_source_id_assigner(&config.source_id_key);
    let index_start_dt = record_manager.aget_time().await?;

    let mut num_added: usize = 0;
    let mut num_skipped: usize = 0;
    let mut num_updated: usize = 0;
    let mut num_deleted: usize = 0;
    let mut scoped_full_cleanup_source_ids: HashSet<String> = HashSet::new();

    let doc_batches = batch_iter(config.batch_size, docs_source);

    for doc_batch in doc_batches {
        let original_batch_size = doc_batch.len();

        let hashed_docs: Vec<Document> = doc_batch
            .iter()
            .map(|doc| get_document_with_hash(doc, &config.key_encoder))
            .collect::<Result<Vec<_>>>()?;
        let hashed_docs = deduplicate_in_order(hashed_docs);

        num_skipped += original_batch_size - hashed_docs.len();

        let source_ids: Vec<Option<String>> = hashed_docs.iter().map(&source_id_assigner).collect();

        if matches!(
            config.cleanup,
            Some(CleanupMode::Incremental) | Some(CleanupMode::ScopedFull)
        ) {
            for (source_id, hashed_doc) in source_ids.iter().zip(&hashed_docs) {
                if source_id.is_none() {
                    return Err(Error::InvalidConfig(format!(
                        "Source IDs are required when cleanup mode is incremental or scoped_full. \
                         Document that starts with content: {} was not assigned as source id.",
                        &hashed_doc.page_content[..hashed_doc.page_content.len().min(100)]
                    )));
                }
                if config.cleanup == Some(CleanupMode::ScopedFull) {
                    scoped_full_cleanup_source_ids.insert(source_id.clone().unwrap_or_default());
                }
            }
        }

        let doc_ids: Vec<String> = hashed_docs
            .iter()
            .map(|doc| {
                doc.id
                    .clone()
                    .ok_or_else(|| Error::Indexing("hash should have set document id".to_string()))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let exists_batch = record_manager.aexists(&doc_ids).await?;

        let mut uids = Vec::new();
        let mut docs_to_index = Vec::new();
        let mut uids_to_refresh = Vec::new();
        let mut seen_docs: HashSet<String> = HashSet::new();

        for (hashed_doc, doc_exists) in hashed_docs.into_iter().zip(exists_batch) {
            let hashed_id = hashed_doc
                .id
                .clone()
                .ok_or_else(|| Error::Indexing("hash should have set document id".to_string()))?;
            if doc_exists {
                if config.force_update {
                    seen_docs.insert(hashed_id.clone());
                } else {
                    uids_to_refresh.push(hashed_id);
                    continue;
                }
            }
            uids.push(hashed_id);
            docs_to_index.push(hashed_doc);
        }

        if !uids_to_refresh.is_empty() {
            record_manager
                .aupdate(&uids_to_refresh, None, Some(index_start_dt))
                .await?;
            num_skipped += uids_to_refresh.len();
        }

        if !docs_to_index.is_empty() {
            aadd_to_destination(destination, docs_to_index.clone(), uids.clone()).await?;
            num_added += docs_to_index.len() - seen_docs.len();
            num_updated += seen_docs.len();
        }

        let group_ids_for_update: Vec<Option<String>> = source_ids;
        record_manager
            .aupdate(&doc_ids, Some(&group_ids_for_update), Some(index_start_dt))
            .await?;

        if config.cleanup == Some(CleanupMode::Incremental) {
            let source_ids_for_cleanup: Vec<String> = group_ids_for_update
                .iter()
                .filter_map(|s| s.clone())
                .collect();

            loop {
                let uids_to_delete = record_manager
                    .alist_keys(
                        Some(index_start_dt),
                        None,
                        Some(&source_ids_for_cleanup),
                        Some(config.cleanup_batch_size),
                    )
                    .await?;
                if uids_to_delete.is_empty() {
                    break;
                }
                adelete_from_destination(destination, &uids_to_delete).await?;
                record_manager.adelete_keys(&uids_to_delete).await?;
                num_deleted += uids_to_delete.len();
            }
        }
    }

    if config.cleanup == Some(CleanupMode::Full)
        || (config.cleanup == Some(CleanupMode::ScopedFull)
            && !scoped_full_cleanup_source_ids.is_empty())
    {
        let delete_group_ids: Option<Vec<String>> =
            if config.cleanup == Some(CleanupMode::ScopedFull) {
                Some(scoped_full_cleanup_source_ids.into_iter().collect())
            } else {
                None
            };

        loop {
            let uids_to_delete = record_manager
                .alist_keys(
                    Some(index_start_dt),
                    None,
                    delete_group_ids.as_deref(),
                    Some(config.cleanup_batch_size),
                )
                .await?;
            if uids_to_delete.is_empty() {
                break;
            }
            adelete_from_destination(destination, &uids_to_delete).await?;
            record_manager.adelete_keys(&uids_to_delete).await?;
            num_deleted += uids_to_delete.len();
        }
    }

    Ok(IndexingResult {
        num_added,
        num_updated,
        num_skipped,
        num_deleted,
    })
}
