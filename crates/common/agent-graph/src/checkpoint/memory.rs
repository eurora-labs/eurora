//! In-memory checkpoint saver for agent state persistence.
//!
//! This module provides an in-memory implementation of a checkpoint saver
//! that stores agent states keyed by thread ID and checkpoint namespace.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Serialize, de::DeserializeOwned};

/// In-memory checkpoint saver for persisting agent state.
///
/// This checkpoint saver stores checkpoints in memory using a `HashMap`.
/// Each thread (thread) has its own checkpoint history.
///
/// Note: Only use `InMemorySaver` for debugging or testing purposes.
/// For production use cases, consider implementing a persistent store.
///
/// # Example
///
/// ```ignore
/// use agent_graph::checkpoint::InMemorySaver;
///
/// let saver = InMemorySaver::new();
///
/// // Save a checkpoint
/// saver.put("thread-1", "state", &my_state);
///
/// // Retrieve a checkpoint
/// let state: Option<MyState> = saver.get("thread-1", "state");
/// ```
#[derive(Clone)]
pub struct InMemorySaver {
    /// Thread ID -> Checkpoint NS -> Checkpoint ID -> serialized data
    #[allow(clippy::type_complexity)]
    storage: Arc<RwLock<HashMap<String, HashMap<String, Vec<u8>>>>>,
}

impl Default for InMemorySaver {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemorySaver {
    /// Create a new in-memory checkpoint saver.
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Save a checkpoint for a given thread and key.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - The thread identifier (e.g., thread ID).
    /// * `key` - The checkpoint key (e.g., "messages", "state").
    /// * `value` - The value to store (must implement Serialize).
    ///
    /// # Panics
    ///
    /// Panics if serialization fails or if the lock is poisoned.
    pub fn put<T: Serialize>(&self, thread_id: &str, key: &str, value: &T) {
        let data = serde_json::to_vec(value).expect("Failed to serialize checkpoint");
        let mut storage = self.storage.write().expect("Lock poisoned");
        storage
            .entry(thread_id.to_string())
            .or_default()
            .insert(key.to_string(), data);
    }

    /// Retrieve a checkpoint for a given thread and key.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - The thread identifier.
    /// * `key` - The checkpoint key.
    ///
    /// # Returns
    ///
    /// `Some(value)` if the checkpoint exists and can be deserialized,
    /// `None` otherwise.
    pub fn get<T: DeserializeOwned>(&self, thread_id: &str, key: &str) -> Option<T> {
        let storage = self.storage.read().expect("Lock poisoned");
        storage
            .get(thread_id)
            .and_then(|thread| thread.get(key))
            .and_then(|data| serde_json::from_slice(data).ok())
    }

    /// Check if a checkpoint exists for a given thread and key.
    pub fn has(&self, thread_id: &str, key: &str) -> bool {
        let storage = self.storage.read().expect("Lock poisoned");
        storage
            .get(thread_id)
            .is_some_and(|thread| thread.contains_key(key))
    }

    /// Delete a checkpoint for a given thread and key.
    pub fn delete(&self, thread_id: &str, key: &str) -> bool {
        let mut storage = self.storage.write().expect("Lock poisoned");
        storage
            .get_mut(thread_id)
            .is_some_and(|thread| thread.remove(key).is_some())
    }

    /// Delete all checkpoints for a given thread.
    pub fn delete_thread(&self, thread_id: &str) -> bool {
        let mut storage = self.storage.write().expect("Lock poisoned");
        storage.remove(thread_id).is_some()
    }

    /// List all thread IDs with checkpoints.
    pub fn list_threads(&self) -> Vec<String> {
        let storage = self.storage.read().expect("Lock poisoned");
        storage.keys().cloned().collect()
    }

    /// List all checkpoint keys for a given thread.
    pub fn list_keys(&self, thread_id: &str) -> Vec<String> {
        let storage = self.storage.read().expect("Lock poisoned");
        storage
            .get(thread_id)
            .map(|thread| thread.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl std::fmt::Debug for InMemorySaver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let storage = self.storage.read().expect("Lock poisoned");
        f.debug_struct("InMemorySaver")
            .field("threads", &storage.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestState {
        messages: Vec<String>,
        count: u32,
    }

    #[test]
    fn test_put_and_get() {
        let saver = InMemorySaver::new();
        let state = TestState {
            messages: vec!["hello".to_string()],
            count: 1,
        };

        saver.put("thread-1", "state", &state);
        let retrieved: Option<TestState> = saver.get("thread-1", "state");

        assert_eq!(retrieved, Some(state));
    }

    #[test]
    fn test_get_nonexistent() {
        let saver = InMemorySaver::new();
        let retrieved: Option<TestState> = saver.get("thread-1", "state");
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_has() {
        let saver = InMemorySaver::new();
        let state = TestState {
            messages: vec![],
            count: 0,
        };

        assert!(!saver.has("thread-1", "state"));
        saver.put("thread-1", "state", &state);
        assert!(saver.has("thread-1", "state"));
    }

    #[test]
    fn test_delete() {
        let saver = InMemorySaver::new();
        let state = TestState {
            messages: vec![],
            count: 0,
        };

        saver.put("thread-1", "state", &state);
        assert!(saver.has("thread-1", "state"));

        assert!(saver.delete("thread-1", "state"));
        assert!(!saver.has("thread-1", "state"));
    }

    #[test]
    fn test_multiple_threads() {
        let saver = InMemorySaver::new();

        let state1 = TestState {
            messages: vec!["thread1".to_string()],
            count: 1,
        };
        let state2 = TestState {
            messages: vec!["thread2".to_string()],
            count: 2,
        };

        saver.put("thread-1", "state", &state1);
        saver.put("thread-2", "state", &state2);

        let retrieved1: Option<TestState> = saver.get("thread-1", "state");
        let retrieved2: Option<TestState> = saver.get("thread-2", "state");

        assert_eq!(retrieved1, Some(state1));
        assert_eq!(retrieved2, Some(state2));
    }
}
