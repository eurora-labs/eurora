use agent_chain_core::messages::{ContentBlocks, PlainTextContentBlock};
use async_trait::async_trait;
use euro_native_messaging::NativeArticleAsset;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    ActivityResult, error::ActivityError, storage::SaveableAsset, types::AssetFunctionality,
};

const ARTICLE_EXTENSION_ID: &str = "309f0906-d48c-4439-9751-7bcf915cdfc5";

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

    fn construct_messages(&self) -> ContentBlocks {
        let asset_json = serde_json::to_string(&self).unwrap_or_default();

        let extras = HashMap::from([(
            "asset_id".to_string(),
            serde_json::json!(ARTICLE_EXTENSION_ID),
        )]);

        let block = PlainTextContentBlock::builder()
            .context(format!("Content of the website titled: '{}'", self.title))
            .title(format!("{}.json", self.title))
            .mime_type("application/json".to_string())
            .text(asset_json)
            .extras(extras)
            .build();

        vec![block.into()].into()
    }

    fn get_id(&self) -> &str {
        &self.id
    }
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
    use agent_chain_core::messages::ContentBlock;

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
    fn trait_methods_work() {
        use crate::types::AssetFunctionality;
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            "This is a test article with some content.".to_string(),
        );
        let blocks = AssetFunctionality::construct_messages(&asset);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
    }
}
