//! Default asset implementation for unsupported activity types

use crate::storage::SaveableAsset;
use crate::types::{CommonFunctionality, ContextChip, SaveFunctionality};
use crate::{AssetStorage, SavedAssetInfo};
use async_trait::async_trait;
use ferrous_llm_core::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};

/// Default asset for activities that don't have specific implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultAsset {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl DefaultAsset {
    /// Create a new default asset
    pub fn new(
        id: String,
        name: String,
        icon: Option<String>,
        description: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            icon,
            description,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a simple default asset with just a name
    pub fn simple(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            icon: None,
            description: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to the asset
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add multiple metadata entries
    pub fn with_metadata_map(
        mut self,
        metadata: std::collections::HashMap<String, String>,
    ) -> Self {
        self.metadata.extend(metadata);
        self
    }

    /// Get a specific metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Check if the asset has a specific metadata key
    pub fn has_metadata(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }

    /// Get all metadata keys
    pub fn get_metadata_keys(&self) -> Vec<&String> {
        self.metadata.keys().collect()
    }

    /// Update the description
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    /// Update the icon
    pub fn set_icon(&mut self, icon: String) {
        self.icon = Some(icon);
    }
}

#[async_trait]
impl SaveFunctionality for DefaultAsset {
    async fn save_to_disk(&self, storage: &AssetStorage) -> crate::error::Result<SavedAssetInfo> {
        storage.save_asset(self).await
    }
}

impl CommonFunctionality for DefaultAsset {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_icon(&self) -> Option<&str> {
        Some("Default icon")
    }

    /// Construct a message for LLM interaction
    fn construct_message(&self) -> Message {
        let mut content = format!("I am working with an application called '{}'", self.name);

        if let Some(description) = &self.description {
            content.push_str(&format!(" - {}", description));
        }

        if !self.metadata.is_empty() {
            content.push_str(" with the following context:");
            for (key, value) in &self.metadata {
                content.push_str(&format!("\n- {}: {}", key, value));
            }
        }

        content.push_str(" and have a question about it.");

        Message {
            role: Role::User,
            content: MessageContent::Text(content),
        }
    }

    /// Get context chip for UI integration (returns None for default assets)
    fn get_context_chip(&self) -> Option<ContextChip> {
        None
    }
}

#[async_trait]
impl SaveableAsset for DefaultAsset {
    fn get_asset_type(&self) -> &'static str {
        "default"
    }

    fn get_file_extension(&self) -> &'static str {
        "json"
    }

    fn get_mime_type(&self) -> &'static str {
        "application/json"
    }

    async fn serialize_content(&self) -> crate::error::Result<Vec<u8>> {
        let json = serde_json::to_string_pretty(self)?;
        Ok(json.into_bytes())
    }

    fn get_unique_id(&self) -> String {
        self.id.clone()
    }

    fn get_display_name(&self) -> String {
        self.name.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_asset_creation() {
        let asset = DefaultAsset::new(
            "test-id".to_string(),
            "Test App".to_string(),
            Some("test-icon".to_string()),
            Some("A test application".to_string()),
        );

        assert_eq!(asset.id, "test-id");
        assert_eq!(asset.name, "Test App");
        assert_eq!(asset.icon, Some("test-icon".to_string()));
        assert_eq!(asset.description, Some("A test application".to_string()));
    }

    #[test]
    fn test_simple_default_asset() {
        let asset = DefaultAsset::simple("Simple App".to_string());

        assert_eq!(asset.name, "Simple App");
        assert!(asset.icon.is_none());
        assert!(asset.description.is_none());
        assert!(asset.metadata.is_empty());
        // ID should be generated
        assert!(!asset.id.is_empty());
    }

    #[test]
    fn test_metadata_operations() {
        let mut asset = DefaultAsset::simple("Test App".to_string())
            .with_metadata("version".to_string(), "1.0.0".to_string())
            .with_metadata("author".to_string(), "Test Author".to_string());

        assert_eq!(asset.get_metadata("version"), Some(&"1.0.0".to_string()));
        assert_eq!(
            asset.get_metadata("author"),
            Some(&"Test Author".to_string())
        );
        assert!(asset.has_metadata("version"));
        assert!(!asset.has_metadata("nonexistent"));

        let keys = asset.get_metadata_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&&"version".to_string()));
        assert!(keys.contains(&&"author".to_string()));
    }

    #[test]
    fn test_metadata_map() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        let asset = DefaultAsset::simple("Test App".to_string()).with_metadata_map(metadata);

        assert_eq!(asset.metadata.len(), 2);
        assert_eq!(asset.get_metadata("key1"), Some(&"value1".to_string()));
        assert_eq!(asset.get_metadata("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_message_construction() {
        let asset = DefaultAsset::new(
            "test-id".to_string(),
            "Test App".to_string(),
            None,
            Some("A test application".to_string()),
        )
        .with_metadata("version".to_string(), "1.0.0".to_string())
        .with_metadata("status".to_string(), "active".to_string());

        let message = asset.construct_message();

        match message.content {
            MessageContent::Text(text) => {
                assert!(text.contains("Test App"));
                assert!(text.contains("A test application"));
                assert!(text.contains("version: 1.0.0"));
                assert!(text.contains("status: active"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_setters() {
        let mut asset = DefaultAsset::simple("Test App".to_string());

        asset.set_description("New description".to_string());
        asset.set_icon("new-icon".to_string());

        assert_eq!(asset.description, Some("New description".to_string()));
        assert_eq!(asset.icon, Some("new-icon".to_string()));
    }

    #[test]
    fn test_context_chip() {
        let asset = DefaultAsset::simple("Test App".to_string());
        assert!(asset.get_context_chip().is_none());
    }
}
