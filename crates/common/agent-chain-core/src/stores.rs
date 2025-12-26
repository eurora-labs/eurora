//! **Store** implements the key-value stores and storage helpers.
//!
//! Module provides implementations of various key-value stores that conform
//! to a simple key-value interface.
//!
//! The primary goal of these storages is to support implementation of caching.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::Error;

/// Abstract interface for a key-value store.
///
/// This is an interface that's meant to abstract away the details of
/// different key-value stores. It provides a simple interface for
/// getting, setting, and deleting key-value pairs.
///
/// The basic methods are `mget`, `mset`, and `mdelete` for getting,
/// setting, and deleting multiple key-value pairs at once. The `yield_keys`
/// method is used to iterate over keys that match a given prefix.
///
/// The async versions of these methods are also provided, which are
/// meant to be used in async contexts. The async methods have the same names
/// but return futures.
///
/// By default, the async methods are implemented using the synchronous methods
/// wrapped in tokio's spawn_blocking. If the store can natively support async
/// operations, it should override these methods.
///
/// By design the methods only accept batches of keys and values, and not
/// single keys or values. This is done to force user code to work with batches
/// which will usually be more efficient by saving on round trips to the store.
#[async_trait]
pub trait BaseStore<K, V>: Send + Sync
where
    K: Send + Sync,
    V: Send + Sync + Clone,
{
    /// Get the values associated with the given keys.
    ///
    /// # Arguments
    ///
    /// * `keys` - A sequence of keys.
    ///
    /// # Returns
    ///
    /// A sequence of optional values associated with the keys.
    /// If a key is not found, the corresponding value will be `None`.
    fn mget(&self, keys: &[K]) -> Vec<Option<V>>;

    /// Async get the values associated with the given keys.
    ///
    /// # Arguments
    ///
    /// * `keys` - A sequence of keys.
    ///
    /// # Returns
    ///
    /// A sequence of optional values associated with the keys.
    /// If a key is not found, the corresponding value will be `None`.
    async fn amget(&self, keys: &[K]) -> Vec<Option<V>>
    where
        K: 'static,
        V: 'static,
    {
        self.mget(keys)
    }

    /// Set the values for the given keys.
    ///
    /// # Arguments
    ///
    /// * `key_value_pairs` - A sequence of key-value pairs.
    fn mset(&self, key_value_pairs: &[(K, V)]);

    /// Async set the values for the given keys.
    ///
    /// # Arguments
    ///
    /// * `key_value_pairs` - A sequence of key-value pairs.
    async fn amset(&self, key_value_pairs: &[(K, V)])
    where
        K: 'static,
        V: 'static,
    {
        self.mset(key_value_pairs)
    }

    /// Delete the given keys and their associated values.
    ///
    /// # Arguments
    ///
    /// * `keys` - A sequence of keys to delete.
    fn mdelete(&self, keys: &[K]);

    /// Async delete the given keys and their associated values.
    ///
    /// # Arguments
    ///
    /// * `keys` - A sequence of keys to delete.
    async fn amdelete(&self, keys: &[K])
    where
        K: 'static,
        V: 'static,
    {
        self.mdelete(keys)
    }

    /// Get an iterator over keys that match the given prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to match.
    ///
    /// # Returns
    ///
    /// A vector of keys that match the given prefix.
    fn yield_keys(&self, prefix: Option<&str>) -> Vec<String>;

    /// Async get an iterator over keys that match the given prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to match.
    ///
    /// # Returns
    ///
    /// A vector of keys that match the given prefix.
    async fn ayield_keys(&self, prefix: Option<&str>) -> Vec<String>
    where
        K: 'static,
        V: 'static,
    {
        self.yield_keys(prefix)
    }
}

/// Type alias for a store with string keys and byte values.
pub type ByteStore = dyn BaseStore<String, Vec<u8>>;

/// In-memory implementation of the BaseStore using a dictionary.
///
/// This implementation uses an `Arc<RwLock<HashMap>>` internally to allow
/// for concurrent access and mutation.
pub struct InMemoryBaseStore<V>
where
    V: Clone + Send + Sync,
{
    store: Arc<RwLock<HashMap<String, V>>>,
}

impl<V> Default for InMemoryBaseStore<V>
where
    V: Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V> InMemoryBaseStore<V>
where
    V: Clone + Send + Sync,
{
    /// Initialize an empty store.
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a reference to the internal store for direct access.
    pub fn store(&self) -> &Arc<RwLock<HashMap<String, V>>> {
        &self.store
    }
}

#[async_trait]
impl<V> BaseStore<String, V> for InMemoryBaseStore<V>
where
    V: Clone + Send + Sync + 'static,
{
    fn mget(&self, keys: &[String]) -> Vec<Option<V>> {
        let store = self.store.blocking_read();
        keys.iter().map(|key| store.get(key).cloned()).collect()
    }

    async fn amget(&self, keys: &[String]) -> Vec<Option<V>> {
        let store = self.store.read().await;
        keys.iter().map(|key| store.get(key).cloned()).collect()
    }

    fn mset(&self, key_value_pairs: &[(String, V)]) {
        let mut store = self.store.blocking_write();
        for (key, value) in key_value_pairs {
            store.insert(key.clone(), value.clone());
        }
    }

    async fn amset(&self, key_value_pairs: &[(String, V)]) {
        let mut store = self.store.write().await;
        for (key, value) in key_value_pairs {
            store.insert(key.clone(), value.clone());
        }
    }

    fn mdelete(&self, keys: &[String]) {
        let mut store = self.store.blocking_write();
        for key in keys {
            store.remove(key);
        }
    }

    async fn amdelete(&self, keys: &[String]) {
        let mut store = self.store.write().await;
        for key in keys {
            store.remove(key);
        }
    }

    fn yield_keys(&self, prefix: Option<&str>) -> Vec<String> {
        let store = self.store.blocking_read();
        match prefix {
            None => store.keys().cloned().collect(),
            Some(prefix) => store
                .keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect(),
        }
    }

    async fn ayield_keys(&self, prefix: Option<&str>) -> Vec<String> {
        let store = self.store.read().await;
        match prefix {
            None => store.keys().cloned().collect(),
            Some(prefix) => store
                .keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect(),
        }
    }
}

/// In-memory store for any type of data.
///
/// This is a type alias for `InMemoryBaseStore<serde_json::Value>` which
/// can store any JSON-serializable value.
///
/// # Examples
///
/// ```
/// use agent_chain_core::stores::InMemoryStore;
/// use agent_chain_core::stores::BaseStore;
/// use serde_json::json;
///
/// let store = InMemoryStore::new();
/// store.mset(&[
///     ("key1".to_string(), json!("value1")),
///     ("key2".to_string(), json!("value2")),
/// ]);
///
/// let values = store.mget(&["key1".to_string(), "key2".to_string()]);
/// assert_eq!(values[0], Some(json!("value1")));
/// assert_eq!(values[1], Some(json!("value2")));
///
/// store.mdelete(&["key1".to_string()]);
/// let keys: Vec<String> = store.yield_keys(None);
/// assert_eq!(keys, vec!["key2".to_string()]);
/// ```
pub type InMemoryStore = InMemoryBaseStore<serde_json::Value>;

/// In-memory store for bytes.
///
/// This is a type alias for `InMemoryBaseStore<Vec<u8>>` which stores
/// byte vectors.
///
/// # Examples
///
/// ```
/// use agent_chain_core::stores::InMemoryByteStore;
/// use agent_chain_core::stores::BaseStore;
///
/// let store = InMemoryByteStore::new();
/// store.mset(&[
///     ("key1".to_string(), b"value1".to_vec()),
///     ("key2".to_string(), b"value2".to_vec()),
/// ]);
///
/// let values = store.mget(&["key1".to_string(), "key2".to_string()]);
/// assert_eq!(values[0], Some(b"value1".to_vec()));
/// assert_eq!(values[1], Some(b"value2".to_vec()));
///
/// store.mdelete(&["key1".to_string()]);
/// let keys: Vec<String> = store.yield_keys(None);
/// assert_eq!(keys, vec!["key2".to_string()]);
/// ```
pub type InMemoryByteStore = InMemoryBaseStore<Vec<u8>>;

/// Error raised when a key is invalid; e.g., uses incorrect characters.
#[derive(Debug, Clone)]
pub struct InvalidKeyException {
    /// The invalid key that caused the error.
    pub key: String,
    /// A message describing why the key is invalid.
    pub message: String,
}

impl InvalidKeyException {
    /// Create a new InvalidKeyException.
    pub fn new(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for InvalidKeyException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid key '{}': {}", self.key, self.message)
    }
}

impl std::error::Error for InvalidKeyException {}

impl From<InvalidKeyException> for Error {
    fn from(e: InvalidKeyException) -> Self {
        Error::Other(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_in_memory_store_mget_mset() {
        let store = InMemoryStore::new();

        store.mset(&[
            ("key1".to_string(), json!("value1")),
            ("key2".to_string(), json!(42)),
        ]);

        let values = store.mget(&["key1".to_string(), "key2".to_string(), "key3".to_string()]);
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], Some(json!("value1")));
        assert_eq!(values[1], Some(json!(42)));
        assert_eq!(values[2], None);
    }

    #[test]
    fn test_in_memory_store_mdelete() {
        let store = InMemoryStore::new();

        store.mset(&[
            ("key1".to_string(), json!("value1")),
            ("key2".to_string(), json!("value2")),
        ]);

        store.mdelete(&["key1".to_string()]);

        let values = store.mget(&["key1".to_string(), "key2".to_string()]);
        assert_eq!(values[0], None);
        assert_eq!(values[1], Some(json!("value2")));
    }

    #[test]
    fn test_in_memory_store_yield_keys() {
        let store = InMemoryStore::new();

        store.mset(&[
            ("prefix_a".to_string(), json!("a")),
            ("prefix_b".to_string(), json!("b")),
            ("other".to_string(), json!("c")),
        ]);

        let all_keys = store.yield_keys(None);
        assert_eq!(all_keys.len(), 3);

        let mut prefix_keys = store.yield_keys(Some("prefix_"));
        prefix_keys.sort();
        assert_eq!(prefix_keys, vec!["prefix_a", "prefix_b"]);
    }

    #[test]
    fn test_in_memory_byte_store() {
        let store = InMemoryByteStore::new();

        store.mset(&[
            ("key1".to_string(), b"bytes1".to_vec()),
            ("key2".to_string(), b"bytes2".to_vec()),
        ]);

        let values = store.mget(&["key1".to_string(), "key2".to_string()]);
        assert_eq!(values[0], Some(b"bytes1".to_vec()));
        assert_eq!(values[1], Some(b"bytes2".to_vec()));
    }

    #[tokio::test]
    async fn test_in_memory_store_async() {
        let store = InMemoryStore::new();

        store
            .amset(&[
                ("key1".to_string(), json!("async_value1")),
                ("key2".to_string(), json!("async_value2")),
            ])
            .await;

        let values = store.amget(&["key1".to_string(), "key2".to_string()]).await;
        assert_eq!(values[0], Some(json!("async_value1")));
        assert_eq!(values[1], Some(json!("async_value2")));

        store.amdelete(&["key1".to_string()]).await;

        let keys = store.ayield_keys(None).await;
        assert_eq!(keys, vec!["key2".to_string()]);
    }

    #[test]
    fn test_invalid_key_exception() {
        let exception = InvalidKeyException::new("bad/key", "keys cannot contain slashes");
        assert_eq!(
            exception.to_string(),
            "Invalid key 'bad/key': keys cannot contain slashes"
        );
    }
}
