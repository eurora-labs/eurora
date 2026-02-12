use std::collections::HashMap;

use agent_chain_core::{BaseMessage, HumanMessage};
use async_trait::async_trait;
use euro_native_messaging::{NativeTwitterAsset, NativeTwitterTweet};
use serde::{Deserialize, Serialize};

use crate::{
    ActivityResult,
    error::ActivityError,
    storage::SaveableAsset,
    types::{AssetFunctionality, ContextChip},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterTweet {
    pub text: String,
    pub timestamp: Option<String>,
    pub author: Option<String>,
    pub likes: Option<u32>,
    pub retweets: Option<u32>,
    pub replies: Option<u32>,
}

impl TwitterTweet {
    pub fn new(text: String, author: Option<String>, timestamp: Option<String>) -> Self {
        Self {
            text,
            timestamp,
            author,
            likes: None,
            retweets: None,
            replies: None,
        }
    }

    pub fn get_formatted_text(&self) -> String {
        if let Some(author) = &self.author {
            format!("@{}: {}", author, self.text)
        } else {
            self.text.clone()
        }
    }

    pub fn contains_hashtag(&self, hashtag: &str) -> bool {
        let hashtag_with_hash = if hashtag.starts_with('#') {
            hashtag.to_string()
        } else {
            format!("#{}", hashtag)
        };
        self.text
            .to_lowercase()
            .contains(&hashtag_with_hash.to_lowercase())
    }

    pub fn extract_hashtags(&self) -> Vec<String> {
        self.text
            .split_whitespace()
            .filter(|word| word.starts_with('#'))
            .map(|hashtag| hashtag.to_string())
            .collect()
    }

    pub fn extract_mentions(&self) -> Vec<String> {
        self.text
            .split_whitespace()
            .filter(|word| word.starts_with('@'))
            .map(|mention| mention.trim_start_matches('@').to_string())
            .collect()
    }
}

impl AssetFunctionality for TwitterAsset {
    fn get_name(&self) -> &str {
        &self.title
    }

    fn get_icon(&self) -> Option<&str> {
        Some("twitter")
    }

    fn construct_messages(&self) -> Vec<BaseMessage> {
        let max_tweets = 20usize;
        let tweet_texts: Vec<String> = self
            .tweets
            .iter()
            .take(max_tweets)
            .map(|tweet| tweet.get_formatted_text())
            .collect();

        let context_description = match self.context_type {
            TwitterContextType::Timeline => "timeline",
            TwitterContextType::Profile => "profile",
            TwitterContextType::Thread => "thread",
            TwitterContextType::Search => "search results",
            TwitterContextType::Hashtag => "hashtag feed",
            TwitterContextType::Other => "other",
        };

        let mut text = format!(
            "The user is looking at Twitter {} content titled '{}' and has a question about it. \
                         Here are the tweets they're seeing: \n\n{}",
            context_description,
            self.title,
            tweet_texts.join("\n\n")
        );
        if self.tweets.len() > max_tweets {
            text.push_str(&format!(
                "\n\n(+{} more tweets truncated)",
                self.tweets.len() - max_tweets,
            ));
        }

        vec![HumanMessage::builder().content(text).build().into()]
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        Some(ContextChip {
            id: self.id.clone(),
            name: "twitter".to_string(),
            extension_id: "2c434895-d32c-485f-8525-c4394863b83a".to_string(),
            attrs: HashMap::new(),
            icon: None,
            position: Some(0),
        })
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

impl From<NativeTwitterTweet> for TwitterTweet {
    fn from(tweet: NativeTwitterTweet) -> Self {
        TwitterTweet {
            text: tweet.text,
            timestamp: tweet.timestamp,
            author: tweet.author,
            likes: None,
            retweets: None,
            replies: None,
        }
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
    Hashtag,
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

    pub fn try_from(asset: NativeTwitterAsset) -> Result<Self, ActivityError> {
        let tweets: Vec<TwitterTweet> = asset.tweets.into_iter().map(TwitterTweet::from).collect();

        Ok(TwitterAsset {
            id: uuid::Uuid::new_v4().to_string(),
            url: asset.url,
            title: asset.title,
            tweets,
            timestamp: asset.timestamp,
            context_type: TwitterContextType::Timeline,
        })
    }

    pub fn get_all_hashtags(&self) -> Vec<String> {
        let mut hashtags = Vec::new();
        for tweet in &self.tweets {
            hashtags.extend(tweet.extract_hashtags());
        }
        hashtags.sort();
        hashtags.dedup();
        hashtags
    }

    pub fn get_all_mentions(&self) -> Vec<String> {
        let mut mentions = Vec::new();
        for tweet in &self.tweets {
            mentions.extend(tweet.extract_mentions());
        }
        mentions.sort();
        mentions.dedup();
        mentions
    }

    pub fn get_tweets_by_author(&self, author: &str) -> Vec<&TwitterTweet> {
        self.tweets
            .iter()
            .filter(|tweet| {
                tweet
                    .author
                    .as_ref()
                    .is_some_and(|a| a.eq_ignore_ascii_case(author))
            })
            .collect()
    }

    pub fn search_tweets(&self, query: &str) -> Vec<&TwitterTweet> {
        let query_lower = query.to_lowercase();
        self.tweets
            .iter()
            .filter(|tweet| tweet.text.to_lowercase().contains(&query_lower))
            .collect()
    }

    pub fn get_tweet_count(&self) -> usize {
        self.tweets.len()
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
    fn test_twitter_tweet_creation() {
        let tweet = TwitterTweet::new(
            "Hello #world from @rust_lang!".to_string(),
            Some("testuser".to_string()),
            Some("2024-01-01T00:00:00Z".to_string()),
        );

        assert_eq!(tweet.text, "Hello #world from @rust_lang!");
        assert_eq!(tweet.author, Some("testuser".to_string()));
        assert_eq!(
            tweet.get_formatted_text(),
            "@testuser: Hello #world from @rust_lang!"
        );
    }

    #[test]
    fn test_hashtag_extraction() {
        let tweet = TwitterTweet::new(
            "Learning #rust and #programming today! #coding".to_string(),
            None,
            None,
        );

        let hashtags = tweet.extract_hashtags();
        assert_eq!(hashtags, vec!["#rust", "#programming", "#coding"]);

        assert!(tweet.contains_hashtag("rust"));
        assert!(tweet.contains_hashtag("#programming"));
        assert!(!tweet.contains_hashtag("python"));
    }

    #[test]
    fn test_mention_extraction() {
        let tweet = TwitterTweet::new(
            "Thanks @rust_lang and @github for the great tools!".to_string(),
            None,
            None,
        );

        let mentions = tweet.extract_mentions();
        assert_eq!(mentions, vec!["rust_lang", "github"]);
    }

    #[test]
    fn test_twitter_asset_creation() {
        let tweets = vec![
            TwitterTweet::new("First tweet".to_string(), Some("user1".to_string()), None),
            TwitterTweet::new(
                "Second tweet #test".to_string(),
                Some("user2".to_string()),
                None,
            ),
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
        assert_eq!(asset.get_tweet_count(), 2);
    }

    #[test]
    fn test_hashtag_aggregation() {
        let tweets = vec![
            TwitterTweet::new(
                "Learning #rust today".to_string(),
                Some("user1".to_string()),
                None,
            ),
            TwitterTweet::new(
                "More #rust and #programming".to_string(),
                Some("user2".to_string()),
                None,
            ),
        ];

        let asset = TwitterAsset::new(
            "test-id".to_string(),
            "https://twitter.com/timeline".to_string(),
            "Timeline".to_string(),
            tweets,
            TwitterContextType::Timeline,
        );

        let hashtags = asset.get_all_hashtags();
        assert_eq!(hashtags, vec!["#programming", "#rust"]);
    }

    #[test]
    fn test_tweet_search() {
        let tweets = vec![
            TwitterTweet::new(
                "Learning Rust programming".to_string(),
                Some("user1".to_string()),
                None,
            ),
            TwitterTweet::new(
                "Python is also great".to_string(),
                Some("user2".to_string()),
                None,
            ),
        ];

        let asset = TwitterAsset::new(
            "test-id".to_string(),
            "https://twitter.com/timeline".to_string(),
            "Timeline".to_string(),
            tweets,
            TwitterContextType::Timeline,
        );

        let rust_tweets = asset.search_tweets("rust");
        assert_eq!(rust_tweets.len(), 1);
        assert!(rust_tweets[0].text.contains("Rust"));

        let programming_tweets = asset.search_tweets("programming");
        assert_eq!(programming_tweets.len(), 1);
    }

    #[test]
    fn test_context_chip() {
        let asset = TwitterAsset::new(
            "test-id".to_string(),
            "https://twitter.com/timeline".to_string(),
            "Timeline".to_string(),
            vec![],
            TwitterContextType::Timeline,
        );

        let chip = asset.get_context_chip().unwrap();
        assert_eq!(chip.id, "test-id");
        assert_eq!(chip.name, "twitter");
        assert_eq!(chip.extension_id, "2c434895-d32c-485f-8525-c4394863b83a");
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
        let messages = AssetFunctionality::construct_messages(&asset);
        let msg = messages[0].clone();
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(matches!(msg, BaseMessage::Human(_)));
        assert!(chip.is_some());
    }
}
