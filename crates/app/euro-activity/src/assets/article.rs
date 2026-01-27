//! Article asset implementation

use std::collections::HashMap;

use agent_chain_core::{BaseMessage, SystemMessage};
use async_trait::async_trait;
use euro_native_messaging::NativeArticleAsset;
use serde::{Deserialize, Serialize};

use crate::{
    ActivityResult,
    error::ActivityError,
    storage::SaveableAsset,
    types::{AssetFunctionality, ContextChip},
};

/// Article asset with content and metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArticleAsset {
    pub id: String,
    pub url: String,
    pub title: String,
    pub content: String,
    pub author: Option<String>,
    pub published_date: Option<String>,
    pub word_count: usize,
}

impl ArticleAsset {
    /// Create a new article asset
    pub fn new(
        id: String,
        url: String,
        title: String,
        content: String,
        author: Option<String>,
        published_date: Option<String>,
    ) -> Self {
        let word_count = content.split_whitespace().count();
        Self {
            id,
            url,
            title,
            content,
            author,
            published_date,
            word_count,
        }
    }

    /// Try to create from protocol buffer state
    pub fn try_from(asset: NativeArticleAsset) -> Result<Self, ActivityError> {
        let word_count = asset.content.split_whitespace().count();
        Ok(ArticleAsset {
            id: uuid::Uuid::new_v4().to_string(),
            url: asset.url,
            title: if asset.title.is_empty() {
                "Article".to_string()
            } else {
                asset.title
            },
            content: asset.text_content,
            author: None,
            published_date: None,
            word_count,
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

    /// Construct a message for LLM interaction
    fn construct_messages(&self) -> Vec<BaseMessage> {
        let mut content = format!(
            "The user is reading an article titled '{}' and has a question about it.",
            self.title
        );

        if let Some(author) = &self.author {
            content.push_str(&format!(" The article is by {}.", author));
        }

        content.push_str(&format!(
            " Here's the text content of the article: \n {}",
            self.content
        ));

        vec![SystemMessage::new(content).into()]
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        // info!("Getting context chip for article: {:?}", &self.url);
        let parsed_url = url::Url::parse(&self.url).ok()?;
        let domain = parsed_url.host_str().unwrap_or_default().to_string();
        // Take title between - and :
        // let title = self.title.clone();
        // let title = title.split('-').nth(1)?.trim().to_string();
        // let title = title.split(':').nth(0)?.trim().to_string();
        Some(ContextChip {
            id: self.id.clone(),
            // name: "article".to_string(),
            name: domain,
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
            Some("Test Author".to_string()),
            Some("2024-01-01".to_string()),
        );

        assert_eq!(asset.id, "test-id");
        assert_eq!(asset.title, "Test Article");
        assert_eq!(asset.author, Some("Test Author".to_string()));
        assert_eq!(asset.word_count, 8_usize);
    }

    #[test]
    fn test_word_count() {
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            "One two three four five".to_string(),
            None,
            None,
        );

        assert_eq!(asset.word_count, 5);
    }

    #[test]
    fn test_context_chip() {
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            "Content".to_string(),
            None,
            None,
        );

        let chip = asset.get_context_chip().unwrap();
        assert_eq!(chip.id, "test-id");
        assert_eq!(chip.name, "example.com");
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
            Some("Test Author".to_string()),
            Some("2024-01-01".to_string()),
        );
        let messages = AssetFunctionality::construct_messages(&asset);
        let msg = messages[0].clone();
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(matches!(msg, BaseMessage::System(_)));
        assert!(chip.is_some());
    }
}
