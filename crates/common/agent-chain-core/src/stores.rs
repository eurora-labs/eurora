use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::Error;

#[async_trait]
pub trait BaseStore<K, V>: Send + Sync
where
    K: Send + Sync,
    V: Send + Sync + Clone,
{
    fn mget(&self, keys: &[K]) -> Vec<Option<V>>;

    async fn amget(&self, keys: &[K]) -> Vec<Option<V>>
    where
        K: 'static,
        V: 'static,
    {
        self.mget(keys)
    }

    fn mset(&self, key_value_pairs: &[(K, V)]);

    async fn amset(&self, key_value_pairs: &[(K, V)])
    where
        K: 'static,
        V: 'static,
    {
        self.mset(key_value_pairs)
    }

    fn mdelete(&self, keys: &[K]);

    async fn amdelete(&self, keys: &[K])
    where
        K: 'static,
        V: 'static,
    {
        self.mdelete(keys)
    }

    fn yield_keys(&self, prefix: Option<&str>) -> Vec<String>;

    async fn ayield_keys(&self, prefix: Option<&str>) -> Vec<String>
    where
        K: 'static,
        V: 'static,
    {
        self.yield_keys(prefix)
    }
}

pub type ByteStore = dyn BaseStore<String, Vec<u8>>;

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
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

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

pub type InMemoryStore = InMemoryBaseStore<serde_json::Value>;

pub type InMemoryByteStore = InMemoryBaseStore<Vec<u8>>;

#[derive(Debug, Clone)]
pub struct InvalidKeyException {
    pub key: String,
    pub message: String,
}

impl InvalidKeyException {
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
