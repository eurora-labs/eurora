use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::documents::Document;
use crate::error::{Error, Result};

/// Internal record stored by the record manager.
#[derive(Debug, Clone)]
pub(crate) struct Record {
    pub group_id: Option<String>,
    pub updated_at: f64,
}

/// Response from an upsert operation.
#[derive(Debug, Clone, Default)]
pub struct UpsertResponse {
    pub succeeded: Vec<String>,
    pub failed: Vec<String>,
}

/// Response from a delete operation. All fields are optional.
#[derive(Debug, Clone, Default)]
pub struct DeleteResponse {
    pub num_deleted: Option<usize>,
    pub succeeded: Option<Vec<String>>,
    pub failed: Option<Vec<String>>,
    pub num_failed: Option<usize>,
}

/// Abstract interface for tracking indexed documents.
///
/// Mirrors `langchain_core.indexing.base.RecordManager`.
#[async_trait]
pub trait RecordManager: Send + Sync {
    /// Get the namespace for this record manager.
    fn namespace(&self) -> &str;

    /// Create the database schema for the record manager.
    fn create_schema(&self) -> Result<()>;

    /// Async create the database schema.
    async fn acreate_schema(&self) -> Result<()> {
        self.create_schema()
    }

    /// Get the current server time as a high resolution timestamp.
    fn get_time(&self) -> Result<f64>;

    /// Async get the current server time.
    async fn aget_time(&self) -> Result<f64> {
        self.get_time()
    }

    /// Upsert records into the database.
    fn update(
        &self,
        keys: &[String],
        group_ids: Option<&[Option<String>]>,
        time_at_least: Option<f64>,
    ) -> Result<()>;

    /// Async upsert records.
    async fn aupdate(
        &self,
        keys: &[String],
        group_ids: Option<&[Option<String>]>,
        time_at_least: Option<f64>,
    ) -> Result<()> {
        self.update(keys, group_ids, time_at_least)
    }

    /// Check if the provided keys exist in the database.
    fn exists(&self, keys: &[String]) -> Result<Vec<bool>>;

    /// Async check if keys exist.
    async fn aexists(&self, keys: &[String]) -> Result<Vec<bool>> {
        self.exists(keys)
    }

    /// List records matching the given filters.
    fn list_keys(
        &self,
        before: Option<f64>,
        after: Option<f64>,
        group_ids: Option<&[String]>,
        limit: Option<usize>,
    ) -> Result<Vec<String>>;

    /// Async list records.
    async fn alist_keys(
        &self,
        before: Option<f64>,
        after: Option<f64>,
        group_ids: Option<&[String]>,
        limit: Option<usize>,
    ) -> Result<Vec<String>> {
        self.list_keys(before, after, group_ids, limit)
    }

    /// Delete specified records.
    fn delete_keys(&self, keys: &[String]) -> Result<()>;

    /// Async delete specified records.
    async fn adelete_keys(&self, keys: &[String]) -> Result<()> {
        self.delete_keys(keys)
    }
}

/// In-memory implementation of `RecordManager`.
///
/// Mirrors `langchain_core.indexing.base.InMemoryRecordManager`.
pub struct InMemoryRecordManager {
    namespace_value: String,
    records: RwLock<HashMap<String, Record>>,
    time_override: RwLock<Option<f64>>,
}

impl InMemoryRecordManager {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace_value: namespace.into(),
            records: RwLock::new(HashMap::new()),
            time_override: RwLock::new(None),
        }
    }

    /// Set a fixed time for testing purposes. Pass `None` to restore real time.
    pub fn set_time_override(&self, time: Option<f64>) {
        let mut override_guard = self
            .time_override
            .write()
            .expect("time_override lock poisoned");
        *override_guard = time;
    }

    fn current_time(&self) -> f64 {
        let override_guard = self
            .time_override
            .read()
            .expect("time_override lock poisoned");
        if let Some(t) = *override_guard {
            return t;
        }
        drop(override_guard);
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX epoch")
            .as_secs_f64()
    }
}

#[async_trait]
impl RecordManager for InMemoryRecordManager {
    fn namespace(&self) -> &str {
        &self.namespace_value
    }

    fn create_schema(&self) -> Result<()> {
        Ok(())
    }

    fn get_time(&self) -> Result<f64> {
        Ok(self.current_time())
    }

    fn update(
        &self,
        keys: &[String],
        group_ids: Option<&[Option<String>]>,
        time_at_least: Option<f64>,
    ) -> Result<()> {
        if let Some(gids) = group_ids {
            if gids.len() != keys.len() {
                return Err(Error::InvalidConfig(format!(
                    "Number of keys ({}) does not match number of group_ids ({})",
                    keys.len(),
                    gids.len()
                )));
            }
        }

        let current = self.current_time();

        if let Some(time_at_least) = time_at_least {
            if time_at_least > current {
                return Err(Error::InvalidConfig(format!(
                    "time_at_least ({time_at_least}) is in the future (current: {current})"
                )));
            }
        }

        let timestamp = if let Some(time_at_least) = time_at_least {
            current.max(time_at_least)
        } else {
            current
        };

        let mut records = self
            .records
            .write()
            .map_err(|e| Error::Other(format!("Failed to acquire write lock: {e}")))?;

        for (i, key) in keys.iter().enumerate() {
            let group_id = group_ids.and_then(|gids| gids.get(i)).cloned().flatten();
            records.insert(
                key.clone(),
                Record {
                    group_id,
                    updated_at: timestamp,
                },
            );
        }

        Ok(())
    }

    fn exists(&self, keys: &[String]) -> Result<Vec<bool>> {
        let records = self
            .records
            .read()
            .map_err(|e| Error::Other(format!("Failed to acquire read lock: {e}")))?;
        Ok(keys.iter().map(|k| records.contains_key(k)).collect())
    }

    fn list_keys(
        &self,
        before: Option<f64>,
        after: Option<f64>,
        group_ids: Option<&[String]>,
        limit: Option<usize>,
    ) -> Result<Vec<String>> {
        let records = self
            .records
            .read()
            .map_err(|e| Error::Other(format!("Failed to acquire read lock: {e}")))?;

        let mut result: Vec<String> = records
            .iter()
            .filter(|(_, record)| {
                if let Some(before) = before {
                    if record.updated_at >= before {
                        return false;
                    }
                }
                if let Some(after) = after {
                    if record.updated_at <= after {
                        return false;
                    }
                }
                if let Some(group_ids) = group_ids {
                    match &record.group_id {
                        Some(gid) => {
                            if !group_ids.contains(gid) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            })
            .map(|(key, _)| key.clone())
            .collect();

        result.sort();

        if let Some(limit) = limit {
            result.truncate(limit);
        }

        Ok(result)
    }

    fn delete_keys(&self, keys: &[String]) -> Result<()> {
        let mut records = self
            .records
            .write()
            .map_err(|e| Error::Other(format!("Failed to acquire write lock: {e}")))?;
        for key in keys {
            records.remove(key);
        }
        Ok(())
    }
}

/// Abstract interface for a document index that supports upsert, delete, and get.
///
/// Mirrors `langchain_core.indexing.base.DocumentIndex`.
/// Types implementing this trait should also implement `BaseRetriever` independently.
#[async_trait]
pub trait DocumentIndex: Send + Sync {
    /// Upsert documents into the index.
    fn upsert(&self, items: &[Document]) -> Result<UpsertResponse>;

    /// Async upsert documents.
    async fn aupsert(&self, items: &[Document]) -> Result<UpsertResponse> {
        self.upsert(items)
    }

    /// Delete documents by IDs.
    fn delete(&self, ids: Option<&[String]>) -> Result<DeleteResponse>;

    /// Async delete documents by IDs.
    async fn adelete(&self, ids: Option<&[String]>) -> Result<DeleteResponse> {
        self.delete(ids)
    }

    /// Get documents by ID. Returns fewer documents if some IDs are not found.
    fn get(&self, ids: &[String]) -> Result<Vec<Document>>;

    /// Async get documents by ID.
    async fn aget(&self, ids: &[String]) -> Result<Vec<Document>> {
        self.get(ids)
    }
}
