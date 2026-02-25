use std::collections::HashMap;

use agent_chain_core::{BaseMessage, HumanMessage};
use async_trait::async_trait;
use euro_native_messaging::NativeArticleAsset;
use serde::{Deserialize, Serialize};

use crate::{
    ActivityResult,
    error::ActivityError,
    storage::SaveableAsset,
    types::{AssetFunctionality, ContextChip},
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArticleAsset {
    pub id: String,
    pub url: String,
    pub title: String,
    pub content: String,
}

impl ArticleAsset {
    pub fn new(id: String, url: String, title: String, content: String) -> Self {
        Self {
            id,
            url,
            title,
            content,
        }
    }

    pub fn try_from(asset: NativeArticleAsset) -> Result<Self, ActivityError> {
        Ok(ArticleAsset {
            id: uuid::Uuid::new_v4().to_string(),
            url: asset.url,
            title: if asset.title.is_empty() {
                "Article".to_string()
            } else {
                asset.title
            },
            content: asset.text_content,
        })
    }
}

impl AssetFunctionality for ArticleAsset {
    fn get_name(&self) -> &str {
        &self.title
    }

    fn get_icon(&self) -> Option<&str> {
        Some("article")
    }

    fn construct_messages(&self) -> Vec<BaseMessage> {
        let mut content = format!(
            "The user is on a website titled '{}' and has a question about it.",
            self.title
        );

        content.push_str(&format!(
            " Here's the text content of the website: \n {}",
            self.content
        ));

        vec![HumanMessage::builder().content(content).build().into()]
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        let name: String = self.title.chars().take(50).collect();
        Some(ContextChip {
            id: self.id.clone(),
            name,
            extension_id: "309f0906-d48c-4439-9751-7bcf915cdfc5".to_string(),
            attrs: HashMap::new(),
            icon: None,
            position: Some(0),
        })
    }

    fn get_id(&self) -> &str {
        &self.id
    }

    // fn from_native(asset: NativeAsset) -> Self {
    //     match asset {
    //         NativeAsset::Article(article) => Self::from(article),
    //         _ => panic!("Invalid asset type"),
    //     }
    // }
}

#[async_trait]
impl SaveableAsset for ArticleAsset {
    fn get_asset_type(&self) -> &'static str {
        "ArticleAsset"
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

impl From<NativeArticleAsset> for ArticleAsset {
    fn from(asset: NativeArticleAsset) -> Self {
        Self::try_from(asset).expect("Failed to convert NativeArticleAsset to ArticleAsset")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_asset_creation() {
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            "This is a test article with some content.".to_string(),
        );

        assert_eq!(asset.id, "test-id");
        assert_eq!(asset.title, "Test Article");
    }

    #[test]
    fn test_context_chip() {
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            "Content".to_string(),
        );

        let chip = asset.get_context_chip().unwrap();
        assert_eq!(chip.id, "test-id");
        assert_eq!(chip.name, "Test Article");
        assert_eq!(chip.extension_id, "309f0906-d48c-4439-9751-7bcf915cdfc5");
    }

    #[test]
    fn trait_methods_work() {
        use crate::types::AssetFunctionality;
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            "This is a test article with some content.".to_string(),
        );
        let messages = AssetFunctionality::construct_messages(&asset);
        let msg = messages[0].clone();
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(matches!(msg, BaseMessage::Human(_)));
        assert!(chip.is_some());
    }
}
