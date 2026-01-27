//! PDF asset implementation

use std::collections::HashMap;

use agent_chain_core::{BaseMessage, SystemMessage};
use async_trait::async_trait;
use euro_native_messaging::NativePdfAsset;
use serde::{Deserialize, Serialize};

use crate::{
    ActivityResult,
    error::ActivityError,
    storage::SaveableAsset,
    types::{AssetFunctionality, ContextChip},
};

/// Pdf asset with content and metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfAsset {
    pub id: String,
    pub url: String,
    pub title: String,
    pub content: String,
}

impl PdfAsset {
    /// Create a new Pdf asset
    pub fn new(id: String, url: String, title: String, content: String) -> Self {
        Self {
            id,
            url,
            title,
            content,
        }
    }

    /// Try to create from protocol buffer state
    pub fn try_from(asset: NativePdfAsset) -> Result<Self, ActivityError> {
        Ok(PdfAsset {
            id: uuid::Uuid::new_v4().to_string(),
            url: asset.url,
            title: if asset.title.is_empty() {
                "Pdf".to_string()
            } else {
                asset.title
            },
            content: asset.content,
        })
    }
}

impl AssetFunctionality for PdfAsset {
    fn get_name(&self) -> &str {
        &self.title
    }

    fn get_icon(&self) -> Option<&str> {
        Some("Pdf")
    }

    /// Construct a message for LLM interaction
    fn construct_messages(&self) -> Vec<BaseMessage> {
        let mut content = format!(
            "The user is reading an Pdf titled '{}' and has a question about it.",
            self.title
        );

        content.push_str(&format!(
            " Here's the text content of the Pdf: \n {}",
            self.content
        ));

        vec![SystemMessage::new(content).into()]
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        let parsed_url = url::Url::parse(&self.url).ok()?;
        let domain = parsed_url.host_str().unwrap_or_default().to_string();
        Some(ContextChip {
            id: self.id.clone(),
            name: domain,
            extension_id: "59b26f84-d10a-11f0-a0a4-17b6bfaafdde".to_string(),
            attrs: HashMap::new(),
            icon: None,
            position: Some(0),
        })
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

#[async_trait]
impl SaveableAsset for PdfAsset {
    fn get_asset_type(&self) -> &'static str {
        "PdfAsset"
    }

    async fn serialize_content(&self) -> ActivityResult<Vec<u8>> {
        let bytes = serde_json::to_vec(&self)?;
        Ok(bytes)
    }

    fn get_unique_id(&self) -> String {
        self.id.clone()
    }

    fn get_display_name(&self) -> String {
        self.title.clone()
    }
}

impl From<NativePdfAsset> for PdfAsset {
    fn from(asset: NativePdfAsset) -> Self {
        Self::try_from(asset).expect("Failed to convert NativePdfAsset to PdfAsset")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_asset_creation() {
        let asset = PdfAsset::new(
            "test-id".to_string(),
            "https://example.com/Pdf".to_string(),
            "Test Pdf".to_string(),
            "This is a test Pdf with some content.".to_string(),
        );

        assert_eq!(asset.id, "test-id");
        assert_eq!(asset.title, "Test Pdf");
    }

    #[test]
    fn test_context_chip() {
        let asset = PdfAsset::new(
            "test-id".to_string(),
            "https://example.com/Pdf".to_string(),
            "Test Pdf".to_string(),
            "Content".to_string(),
        );

        let chip = asset.get_context_chip().unwrap();
        assert_eq!(chip.id, "test-id");
        assert_eq!(chip.name, "example.com");
        assert_eq!(chip.extension_id, "59b26f84-d10a-11f0-a0a4-17b6bfaafdde");
    }

    #[test]
    fn trait_methods_work() {
        use crate::types::AssetFunctionality;
        let asset = PdfAsset::new(
            "test-id".to_string(),
            "https://example.com/Pdf".to_string(),
            "Test Pdf".to_string(),
            "This is a test Pdf with some content.".to_string(),
        );
        let messages = AssetFunctionality::construct_messages(&asset);
        let msg = messages[0].clone();
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(matches!(msg, BaseMessage::System(_)));
        assert!(chip.is_some());
    }
}
