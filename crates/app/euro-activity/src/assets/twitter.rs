use agent_chain_core::messages::{
    ContentBlock, ContentBlocks, ImageContentBlock, PlainTextContentBlock,
};
use async_trait::async_trait;
use euro_native_messaging::{NativeTwitterAsset, NativeTwitterTweet, ParseResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    ActivityResult, error::ActivityError, storage::SaveableAsset, types::AssetFunctionality,
};

const TWITTER_EXTENSION_ID: &str = "2c434895-d32c-485f-8525-c4394863b83a";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterTweet {
    pub text: String,
    pub timestamp: Option<String>,
    pub author: Option<String>,
    #[serde(default)]
    pub images: Vec<String>,
}

impl TwitterTweet {
    pub fn get_formatted_text(&self) -> String {
        let mut text = if let Some(author) = &self.author {
            format!("@{}: {}", author, self.text)
        } else {
            self.text.clone()
        };
        if !self.images.is_empty() {
            text.push_str(&format!("\n[{} image(s) attached]", self.images.len()));
        }
        text
    }
}

impl From<NativeTwitterTweet> for TwitterTweet {
    fn from(tweet: NativeTwitterTweet) -> Self {
        TwitterTweet {
            text: tweet.text,
            timestamp: tweet.timestamp,
            author: tweet.author,
            images: tweet.images,
        }
    }
}

impl AssetFunctionality for TwitterAsset {
    fn get_name(&self) -> &str {
        &self.title
    }

    fn get_icon(&self) -> Option<&str> {
        Some("twitter")
    }

    fn construct_messages(&self) -> ContentBlocks {
        match self.context_type {
            TwitterContextType::Thread => self.construct_thread_messages(),
            _ => self.construct_default_messages(),
        }
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TwitterAsset {
    pub id: String,
    pub url: String,
    pub title: String,
    pub tweets: Vec<TwitterTweet>,
    pub timestamp: String,
    pub context_type: TwitterContextType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TwitterContextType {
    Timeline,
    Profile,
    Thread,
    Search,
    #[default]
    Other,
}

impl TwitterAsset {
    pub fn new(
        id: String,
        url: String,
        title: String,
        tweets: Vec<TwitterTweet>,
        context_type: TwitterContextType,
    ) -> Self {
        Self {
            id,
            url,
            title,
            tweets,
            timestamp: chrono::Utc::now().to_rfc3339(),
            context_type,
        }
    }

    fn asset_extras() -> HashMap<String, serde_json::Value> {
        HashMap::from([(
            "asset_id".to_string(),
            serde_json::json!(TWITTER_EXTENSION_ID),
        )])
    }

    fn construct_thread_messages(&self) -> ContentBlocks {
        let mut blocks: Vec<ContentBlock> = Vec::new();

        let (main_tweet, replies) = match self.tweets.split_first() {
            Some((first, rest)) => (Some(first), rest),
            None => (None, [].as_slice()),
        };

        if let Some(tweet) = main_tweet {
            let author = tweet.author.as_deref().unwrap_or("unknown");
            let tweet_json = serde_json::json!({
                "text": tweet.text,
                "author": tweet.author,
                "timestamp": tweet.timestamp,
            });

            let block = PlainTextContentBlock::builder()
                .context(format!("Twitter thread by @{}", author))
                .title("main_tweet.json".to_string())
                .mime_type("application/json".to_string())
                .text(tweet_json.to_string())
                .extras(Self::asset_extras())
                .build();
            blocks.push(block.into());

            for image in &tweet.images {
                if image.is_empty() {
                    continue;
                }
                match ImageContentBlock::builder()
                    .base64(image.to_string())
                    .mime_type("image/png".to_string())
                    .build()
                {
                    Ok(block) => blocks.push(ContentBlock::Image(block)),
                    Err(e) => tracing::warn!("Failed to create image block: {e}"),
                }
            }
        }

        if !replies.is_empty() {
            let replies_json: Vec<serde_json::Value> = replies
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "text": r.text,
                        "author": r.author,
                        "timestamp": r.timestamp,
                    })
                })
                .collect();

            let block = PlainTextContentBlock::builder()
                .context(format!("Replies ({} total)", replies.len()))
                .title("replies.json".to_string())
                .mime_type("application/json".to_string())
                .text(serde_json::to_string(&replies_json).unwrap_or_default())
                .build();
            blocks.push(block.into());
        }

        blocks.into()
    }

    fn construct_default_messages(&self) -> ContentBlocks {
        let asset_json = serde_json::to_string(&self).unwrap_or_default();

        let context_description = match self.context_type {
            TwitterContextType::Timeline => "timeline",
            TwitterContextType::Profile => "profile",
            TwitterContextType::Search => "search results",
            _ => "other",
        };

        let block = PlainTextContentBlock::builder()
            .context(format!(
                "Twitter {} content titled: '{}'",
                context_description, self.title
            ))
            .title(format!("{}.json", self.title))
            .mime_type("application/json".to_string())
            .text(asset_json)
            .extras(Self::asset_extras())
            .build();

        vec![ContentBlock::from(block)].into()
    }

    pub fn try_from(asset: NativeTwitterAsset) -> Result<Self, ActivityError> {
        let (tweets, context_type) = match asset.result {
            ParseResult::Tweet(data) => {
                let mut tweets: Vec<NativeTwitterTweet> = Vec::new();
                if let Some(tweet) = data.tweet {
                    tweets.push(tweet);
                }
                tweets.extend(data.replies);
                (tweets, TwitterContextType::Thread)
            }
            ParseResult::Profile(data) => (data.tweets, TwitterContextType::Profile),
            ParseResult::Home(data) => (data.tweets, TwitterContextType::Timeline),
            ParseResult::Search(data) => (data.tweets, TwitterContextType::Search),
            ParseResult::Notifications(data) => (data.tweets, TwitterContextType::Other),
            ParseResult::Unsupported(_) => (vec![], TwitterContextType::Other),
        };

        let tweets: Vec<TwitterTweet> = tweets.into_iter().map(TwitterTweet::from).collect();

        Ok(TwitterAsset {
            id: uuid::Uuid::new_v4().to_string(),
            url: asset.url,
            title: asset.title,
            tweets,
            timestamp: asset.timestamp,
            context_type,
        })
    }
}

impl From<NativeTwitterAsset> for TwitterAsset {
    fn from(asset: NativeTwitterAsset) -> Self {
        Self::try_from(asset).expect("Failed to convert NativeTwitterAsset to TwitterAsset")
    }
}

#[async_trait]
impl SaveableAsset for TwitterAsset {
    fn get_asset_type(&self) -> &'static str {
        "TwitterAsset"
    }

    async fn serialize_content(&self) -> ActivityResult<Vec<u8>> {
        let json = serde_json::to_vec(self)?;
        Ok(json)
    }

    fn get_unique_id(&self) -> String {
        self.id.clone()
    }

    fn get_display_name(&self) -> String {
        self.title.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twitter_tweet_formatted_text() {
        let tweet = TwitterTweet {
            text: "Hello world!".to_string(),
            author: Some("testuser".to_string()),
            timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            images: Vec::new(),
        };

        assert_eq!(tweet.get_formatted_text(), "@testuser: Hello world!");
    }

    #[test]
    fn test_twitter_tweet_formatted_text_with_images() {
        let tweet = TwitterTweet {
            text: "Check this out".to_string(),
            author: None,
            timestamp: None,
            images: vec!["img1".to_string(), "img2".to_string()],
        };

        assert_eq!(
            tweet.get_formatted_text(),
            "Check this out\n[2 image(s) attached]"
        );
    }

    #[test]
    fn test_twitter_asset_creation() {
        let tweets = vec![
            TwitterTweet {
                text: "First tweet".to_string(),
                author: Some("user1".to_string()),
                timestamp: None,
                images: Vec::new(),
            },
            TwitterTweet {
                text: "Second tweet".to_string(),
                author: Some("user2".to_string()),
                timestamp: None,
                images: Vec::new(),
            },
        ];

        let asset = TwitterAsset::new(
            "test-id".to_string(),
            "https://twitter.com/timeline".to_string(),
            "My Timeline".to_string(),
            tweets,
            TwitterContextType::Timeline,
        );

        assert_eq!(asset.id, "test-id");
        assert_eq!(asset.title, "My Timeline");
        assert_eq!(asset.tweets.len(), 2);
    }

    #[test]
    fn trait_methods_work() {
        use crate::types::AssetFunctionality;
        let asset = TwitterAsset::new(
            "id".into(),
            "url".into(),
            "title".into(),
            vec![],
            TwitterContextType::Timeline,
        );
        let blocks = AssetFunctionality::construct_messages(&asset);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
    }

    #[test]
    fn thread_produces_separate_blocks() {
        use crate::types::AssetFunctionality;
        let main_tweet = TwitterTweet {
            text: "Main tweet".to_string(),
            author: Some("author".to_string()),
            timestamp: None,
            images: vec!["aW1hZ2VkYXRh".to_string()],
        };
        let reply = TwitterTweet {
            text: "A reply".to_string(),
            author: Some("replier".to_string()),
            timestamp: None,
            images: vec![],
        };
        let asset = TwitterAsset::new(
            "id".into(),
            "url".into(),
            "Thread title".into(),
            vec![main_tweet, reply],
            TwitterContextType::Thread,
        );
        let blocks = AssetFunctionality::construct_messages(&asset);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
        assert!(matches!(blocks[1], ContentBlock::Image(_)));
        assert!(matches!(blocks[2], ContentBlock::PlainText(_)));
    }

    #[test]
    fn thread_no_replies() {
        use crate::types::AssetFunctionality;
        let main_tweet = TwitterTweet {
            text: "Solo tweet".to_string(),
            author: Some("author".to_string()),
            timestamp: None,
            images: vec![],
        };
        let asset = TwitterAsset::new(
            "id".into(),
            "url".into(),
            "title".into(),
            vec![main_tweet],
            TwitterContextType::Thread,
        );
        let blocks = AssetFunctionality::construct_messages(&asset);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
    }
}
