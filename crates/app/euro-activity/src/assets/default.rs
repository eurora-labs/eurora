use agent_chain_core::{BaseMessage, HumanMessage};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    ActivityResult,
    storage::SaveableAsset,
    types::{AssetFunctionality, ContextChip},
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultAsset {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl DefaultAsset {
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

    pub fn simple(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            icon: None,
            description: None,
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl AssetFunctionality for DefaultAsset {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_icon(&self) -> Option<&str> {
        Some("default")
    }

    fn construct_messages(&self) -> Vec<BaseMessage> {
        let mut content = format!(
            "The user is working with an application called '{}'",
            self.name
        );

        if let Some(description) = &self.description {
            content.push_str(&format!(" - {}", description));
        }

        if !self.metadata.is_empty() {
            content.push_str(" with the following context:");
            for (key, value) in &self.metadata {
                content.push_str(&format!("\n- {}: {}", key, value));
            }
        }

        content.push_str(" and has a question about it.");

        vec![HumanMessage::builder().content(content).build().into()]
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        None
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

#[async_trait]
impl SaveableAsset for DefaultAsset {
    fn get_asset_type(&self) -> &'static str {
        "DefaultAsset"
    }

    async fn serialize_content(&self) -> ActivityResult<Vec<u8>> {
        let json = serde_json::to_vec(self)?;
        Ok(json)
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
    fn test_context_chip() {
        let asset = DefaultAsset::simple("Test App".to_string());
        assert!(asset.get_context_chip().is_none());
    }

    #[test]
    fn trait_methods_work() {
        use crate::types::AssetFunctionality;
        let asset = DefaultAsset::new(
            "test-id".to_string(),
            "Test App".to_string(),
            None,
            Some("A test application".to_string()),
        );
        let messages = AssetFunctionality::construct_messages(&asset);
        let msg = messages[0].clone();
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(matches!(msg, BaseMessage::Human(_)));
        assert!(chip.is_none());
    }
}
