use agent_chain_core::messages::{ContentBlocks, PlainTextContentBlock};
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

    fn construct_messages(&self) -> ContentBlocks {
        let asset_json = serde_json::to_string(&self).unwrap_or_default();

        let extras = std::collections::HashMap::from([
            ("type".to_string(), serde_json::json!("asset")),
            ("kind".to_string(), serde_json::json!("default")),
        ]);

        let block = PlainTextContentBlock::builder()
            .context(format!(
                "The user is working with an application called '{}'",
                self.name
            ))
            .title(format!("{}.json", self.name))
            .mime_type("application/json".to_string())
            .text(asset_json)
            .extras(extras)
            .build();

        vec![block.into()].into()
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
    use agent_chain_core::messages::ContentBlock;

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
        let blocks = AssetFunctionality::construct_messages(&asset);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(chip.is_none());
    }
}
