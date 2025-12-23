//! Default snapshot implementation for unsupported activity types

use agent_chain_core::{BaseMessage, HumanMessage};
use serde::{Deserialize, Serialize};

use crate::types::SnapshotFunctionality;

/// Default snapshot for activities that don't have specific implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSnapshot {
    pub id: String,
    pub state: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl DefaultSnapshot {
    /// Create a new default snapshot
    pub fn new(state: String) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            state,
            metadata: std::collections::HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a snapshot with metadata
    pub fn with_metadata(
        state: String,
        metadata: std::collections::HashMap<String, String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            state,
            metadata,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add metadata to the snapshot
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.touch();
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Check if metadata key exists
    pub fn has_metadata(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }

    /// Update the state
    pub fn update_state(&mut self, new_state: String) {
        self.state = new_state;
        self.touch();
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp() as u64;
    }

    /// Get all metadata keys
    pub fn get_metadata_keys(&self) -> Vec<&String> {
        self.metadata.keys().collect()
    }

    /// Clear all metadata
    pub fn clear_metadata(&mut self) {
        self.metadata.clear();
        self.touch();
    }

    /// Merge metadata from another snapshot
    pub fn merge_metadata(&mut self, other: &DefaultSnapshot) {
        for (key, value) in &other.metadata {
            self.metadata.insert(key.clone(), value.clone());
        }
        self.touch();
    }

    /// Check if the snapshot is empty (no state or metadata)
    pub fn is_empty(&self) -> bool {
        self.state.is_empty() && self.metadata.is_empty()
    }

    /// Get a summary of the snapshot
    pub fn get_summary(&self) -> String {
        if self.state.is_empty() && self.metadata.is_empty() {
            "Empty snapshot".to_string()
        } else if self.metadata.is_empty() {
            format!("State: {}", self.state)
        } else {
            format!(
                "State: {} (with {} metadata entries)",
                self.state,
                self.metadata.len()
            )
        }
    }
}

impl SnapshotFunctionality for DefaultSnapshot {
    /// Construct a message for LLM interaction
    fn construct_messages(&self) -> Vec<BaseMessage> {
        let mut content = format!("Current application state: {}", self.state);

        if !self.metadata.is_empty() {
            content.push_str(" with additional context:");
            for (key, value) in &self.metadata {
                content.push_str(&format!("\n- {}: {}", key, value));
            }
        }

        vec![HumanMessage::new(content).into()]
    }

    fn get_updated_at(&self) -> u64 {
        self.updated_at
    }

    fn get_created_at(&self) -> u64 {
        self.created_at
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_snapshot_creation() {
        let snapshot = DefaultSnapshot::new("Active window: Notepad".to_string());

        assert_eq!(snapshot.state, "Active window: Notepad");
        assert!(snapshot.metadata.is_empty());
        assert!(snapshot.created_at > 0);
        assert_eq!(snapshot.created_at, snapshot.updated_at);
    }

    #[test]
    fn test_snapshot_with_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("window_title".to_string(), "Document.txt".to_string());
        metadata.insert("process_id".to_string(), "1234".to_string());

        let snapshot = DefaultSnapshot::with_metadata("Text editor active".to_string(), metadata);

        assert_eq!(snapshot.state, "Text editor active");
        assert_eq!(snapshot.metadata.len(), 2);
        assert_eq!(
            snapshot.get_metadata("window_title"),
            Some(&"Document.txt".to_string())
        );
        assert_eq!(
            snapshot.get_metadata("process_id"),
            Some(&"1234".to_string())
        );
    }

    #[test]
    fn test_metadata_operations() {
        let mut snapshot = DefaultSnapshot::new("Test state".to_string());

        snapshot.add_metadata("key1".to_string(), "value1".to_string());
        snapshot.add_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(snapshot.metadata.len(), 2);
        assert!(snapshot.has_metadata("key1"));
        assert!(snapshot.has_metadata("key2"));
        assert!(!snapshot.has_metadata("key3"));

        let keys = snapshot.get_metadata_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&&"key1".to_string()));
        assert!(keys.contains(&&"key2".to_string()));
    }

    #[test]
    fn test_state_update() {
        let mut snapshot = DefaultSnapshot::new("Initial state".to_string());
        let original_updated_at = snapshot.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        snapshot.update_state("Updated state".to_string());

        assert_eq!(snapshot.state, "Updated state");
        assert!(snapshot.updated_at >= original_updated_at);
    }

    #[test]
    fn test_touch_updates_timestamp() {
        let mut snapshot = DefaultSnapshot::new("Test state".to_string());
        let original_updated_at = snapshot.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        snapshot.touch();

        assert!(snapshot.updated_at >= original_updated_at);
    }

    #[test]
    fn test_clear_metadata() {
        let mut snapshot = DefaultSnapshot::new("Test state".to_string());
        snapshot.add_metadata("key1".to_string(), "value1".to_string());
        snapshot.add_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(snapshot.metadata.len(), 2);

        snapshot.clear_metadata();

        assert!(snapshot.metadata.is_empty());
    }

    #[test]
    fn test_merge_metadata() {
        let mut snapshot1 = DefaultSnapshot::new("State 1".to_string());
        snapshot1.add_metadata("key1".to_string(), "value1".to_string());

        let mut snapshot2 = DefaultSnapshot::new("State 2".to_string());
        snapshot2.add_metadata("key2".to_string(), "value2".to_string());
        snapshot2.add_metadata("key3".to_string(), "value3".to_string());

        snapshot1.merge_metadata(&snapshot2);

        assert_eq!(snapshot1.metadata.len(), 3);
        assert_eq!(snapshot1.get_metadata("key1"), Some(&"value1".to_string()));
        assert_eq!(snapshot1.get_metadata("key2"), Some(&"value2".to_string()));
        assert_eq!(snapshot1.get_metadata("key3"), Some(&"value3".to_string()));
    }

    #[test]
    fn test_is_empty() {
        let empty_snapshot = DefaultSnapshot::new("".to_string());
        assert!(empty_snapshot.is_empty());

        let state_snapshot = DefaultSnapshot::new("Some state".to_string());
        assert!(!state_snapshot.is_empty());

        let mut metadata_snapshot = DefaultSnapshot::new("".to_string());
        metadata_snapshot.add_metadata("key".to_string(), "value".to_string());
        assert!(!metadata_snapshot.is_empty());
    }

    #[test]
    fn test_get_summary() {
        let empty_snapshot = DefaultSnapshot::new("".to_string());
        assert_eq!(empty_snapshot.get_summary(), "Empty snapshot");

        let state_only = DefaultSnapshot::new("Active".to_string());
        assert_eq!(state_only.get_summary(), "State: Active");

        let mut with_metadata = DefaultSnapshot::new("Active".to_string());
        with_metadata.add_metadata("key1".to_string(), "value1".to_string());
        with_metadata.add_metadata("key2".to_string(), "value2".to_string());
        assert_eq!(
            with_metadata.get_summary(),
            "State: Active (with 2 metadata entries)"
        );
    }

    #[test]
    fn test_message_construction() {
        let mut snapshot = DefaultSnapshot::new("Application running".to_string());
        snapshot.add_metadata("version".to_string(), "1.0.0".to_string());
        snapshot.add_metadata("mode".to_string(), "debug".to_string());

        let message = snapshot.construct_messages()[0].clone();
        let text = message.content();

        assert!(text.contains("Application running"));
        assert!(text.contains("version: 1.0.0"));
        assert!(text.contains("mode: debug"));
        assert!(text.contains("additional context"));
    }

    #[test]
    fn test_message_construction_no_metadata() {
        let snapshot = DefaultSnapshot::new("Simple state".to_string());
        let message = snapshot.construct_messages()[0].clone();
        let text = message.content();

        assert_eq!(text, "Current application state: Simple state");
    }
}
