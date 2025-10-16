//! Article asset implementation

use std::collections::HashMap;

use async_trait::async_trait;
use eur_native_messaging::NativeArticleAsset;
use ferrous_llm_core::{Message, MessageContent, Role};
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

    /// Get a preview of the article content (first N words)
    pub fn get_preview(&self, word_limit: usize) -> String {
        let words: Vec<&str> = self.content.split_whitespace().collect();
        if words.len() <= word_limit {
            self.content.clone()
        } else {
            let preview_words = &words[..word_limit];
            format!("{}...", preview_words.join(" "))
        }
    }

    /// Get estimated reading time in minutes
    pub fn get_estimated_reading_time(&self) -> usize {
        // Average reading speed is about 200-250 words per minute
        // Using 225 as a middle ground
        (self.word_count as f64 / 225.0).ceil() as usize
    }

    /// Check if the article contains a specific keyword
    pub fn contains_keyword(&self, keyword: &str) -> bool {
        let keyword_lower = keyword.to_lowercase();
        self.title.to_lowercase().contains(&keyword_lower)
            || self.content.to_lowercase().contains(&keyword_lower)
            || self.author.as_ref().map_or(false, |author| {
                author.to_lowercase().contains(&keyword_lower)
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
    fn construct_messages(&self) -> Vec<Message> {
        let mut content = format!(
            "I am reading an article titled '{}' and have a question about it.",
            self.title
        );

        if let Some(author) = &self.author {
            content.push_str(&format!(" The article is by {}.", author));
        }

        content.push_str(&format!(
            " Here's the text content of the article: \n {}",
            self.content
        ));

        vec![Message {
            role: Role::User,
            content: MessageContent::Text(content),
        }]
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
    fn test_preview() {
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            "This is a long article with many words that should be truncated".to_string(),
            None,
            None,
        );

        let preview = asset.get_preview(5);
        assert_eq!(preview, "This is a long article...");

        let full_preview = asset.get_preview(20);
        assert_eq!(full_preview, asset.content);
    }

    #[test]
    fn test_estimated_reading_time() {
        let short_content = "Short article.".to_string();
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            short_content,
            None,
            None,
        );

        assert_eq!(asset.get_estimated_reading_time(), 1);

        // Test with longer content (450 words should be 2 minutes)
        let long_content = "word ".repeat(450);
        let long_asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Test Article".to_string(),
            long_content,
            None,
            None,
        );

        assert_eq!(long_asset.get_estimated_reading_time(), 2);
    }

    #[test]
    fn test_keyword_search() {
        let asset = ArticleAsset::new(
            "test-id".to_string(),
            "https://example.com/article".to_string(),
            "Rust Programming".to_string(),
            "This article discusses Rust programming language features.".to_string(),
            Some("Jane Doe".to_string()),
            None,
        );

        assert!(asset.contains_keyword("rust"));
        assert!(asset.contains_keyword("Rust"));
        assert!(asset.contains_keyword("programming"));
        assert!(asset.contains_keyword("jane"));
        assert!(!asset.contains_keyword("python"));
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
        let msg = AssetFunctionality::construct_messages(&asset);
        let msg = msg[0].clone();
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(matches!(msg.content, MessageContent::Text(_)));
        assert!(chip.is_some());
    }
}
