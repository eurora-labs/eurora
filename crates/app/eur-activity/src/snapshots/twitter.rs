//! Twitter snapshot implementation

use crate::assets::twitter::TwitterTweet;
use crate::types::SnapshotFunctionality;
use eur_native_messaging::types::NativeTwitterSnapshot;
use ferrous_llm_core::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};

/// Type of Twitter interaction captured in the snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TwitterInteractionType {
    View,
    Like,
    Retweet,
    Reply,
    Quote,
    Follow,
    Bookmark,
}

/// Twitter snapshot with real-time tweet updates and interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterSnapshot {
    pub tweets: Vec<TwitterTweet>,
    pub interaction_type: Option<TwitterInteractionType>,
    pub interaction_target: Option<String>, // Tweet ID or user handle
    pub scroll_position: Option<f32>,
    pub page_context: Option<String>, // timeline, profile, search, etc.
    pub created_at: u64,
    pub updated_at: u64,
}

impl TwitterSnapshot {
    /// Create a new Twitter snapshot
    pub fn new(tweets: Vec<TwitterTweet>) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            tweets,
            interaction_type: None,
            interaction_target: None,
            scroll_position: None,
            page_context: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a snapshot with interaction context
    pub fn with_interaction(
        tweets: Vec<TwitterTweet>,
        interaction_type: TwitterInteractionType,
        interaction_target: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            tweets,
            interaction_type: Some(interaction_type),
            interaction_target,
            scroll_position: None,
            page_context: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a snapshot with full context
    pub fn with_full_context(
        tweets: Vec<TwitterTweet>,
        interaction_type: Option<TwitterInteractionType>,
        interaction_target: Option<String>,
        scroll_position: Option<f32>,
        page_context: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            tweets,
            interaction_type,
            interaction_target,
            scroll_position,
            page_context,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp() as u64;
    }

    /// Get tweet count
    pub fn get_tweet_count(&self) -> usize {
        self.tweets.len()
    }

    /// Check if snapshot has any tweets
    pub fn has_tweets(&self) -> bool {
        !self.tweets.is_empty()
    }

    /// Get all unique hashtags from tweets
    pub fn get_hashtags(&self) -> Vec<String> {
        let mut hashtags = Vec::new();
        for tweet in &self.tweets {
            hashtags.extend(tweet.extract_hashtags());
        }
        hashtags.sort();
        hashtags.dedup();
        hashtags
    }

    /// Get all unique mentions from tweets
    pub fn get_mentions(&self) -> Vec<String> {
        let mut mentions = Vec::new();
        for tweet in &self.tweets {
            mentions.extend(tweet.extract_mentions());
        }
        mentions.sort();
        mentions.dedup();
        mentions
    }

    /// Search tweets containing specific text
    pub fn search_tweets(&self, query: &str) -> Vec<&TwitterTweet> {
        let query_lower = query.to_lowercase();
        self.tweets
            .iter()
            .filter(|tweet| tweet.text.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Filter tweets by author
    pub fn get_tweets_by_author(&self, author: &str) -> Vec<&TwitterTweet> {
        self.tweets
            .iter()
            .filter(|tweet| {
                tweet
                    .author
                    .as_ref()
                    .map_or(false, |a| a.eq_ignore_ascii_case(author))
            })
            .collect()
    }

    /// Check if snapshot represents a specific interaction
    pub fn is_interaction(&self, interaction_type: &TwitterInteractionType) -> bool {
        self.interaction_type.as_ref() == Some(interaction_type)
    }

    /// Get interaction description
    pub fn get_interaction_description(&self) -> Option<String> {
        self.interaction_type.as_ref().map(|interaction| {
            let base_desc = match interaction {
                TwitterInteractionType::View => "Viewing",
                TwitterInteractionType::Like => "Liked",
                TwitterInteractionType::Retweet => "Retweeted",
                TwitterInteractionType::Reply => "Replied to",
                TwitterInteractionType::Quote => "Quote tweeted",
                TwitterInteractionType::Follow => "Followed",
                TwitterInteractionType::Bookmark => "Bookmarked",
            };

            if let Some(target) = &self.interaction_target {
                format!("{} {}", base_desc, target)
            } else {
                base_desc.to_string()
            }
        })
    }
}

impl SnapshotFunctionality for TwitterSnapshot {
    /// Construct a message for LLM interaction
    fn construct_message(&self) -> Message {
        let mut content = String::new();

        // Add context about the page/interaction
        if let Some(context) = &self.page_context {
            content.push_str(&format!("I'm viewing Twitter {} ", context));
        } else {
            content.push_str("I'm viewing Twitter ");
        }

        // Add interaction context
        if let Some(interaction) = &self.interaction_type {
            let interaction_desc = match interaction {
                TwitterInteractionType::View => "viewing",
                TwitterInteractionType::Like => "liking",
                TwitterInteractionType::Retweet => "retweeting",
                TwitterInteractionType::Reply => "replying to",
                TwitterInteractionType::Quote => "quote tweeting",
                TwitterInteractionType::Follow => "following",
                TwitterInteractionType::Bookmark => "bookmarking",
            };
            content.push_str(&format!("and {} ", interaction_desc));

            if let Some(target) = &self.interaction_target {
                content.push_str(&format!("content from {} ", target));
            }
        }

        content.push_str("and have a question about it. Here are the tweets I'm seeing:\n\n");

        // Add tweet content
        let tweet_texts: Vec<String> = self
            .tweets
            .iter()
            .map(|tweet| tweet.get_formatted_text())
            .collect();

        content.push_str(&tweet_texts.join("\n\n"));

        Message {
            role: Role::User,
            content: MessageContent::Text(content),
        }
    }

    fn get_updated_at(&self) -> u64 {
        self.updated_at
    }

    fn get_created_at(&self) -> u64 {
        self.created_at
    }
}

impl From<NativeTwitterSnapshot> for TwitterSnapshot {
    fn from(snapshot: NativeTwitterSnapshot) -> Self {
        // let tweets: Vec<TwitterTweet> = snapshot
        //     .tweets
        //     .into_iter()
        //     .map(TwitterTweet::from)
        //     .collect();
        let tweets = Vec::new();

        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            tweets,
            interaction_type: None,
            interaction_target: None,
            scroll_position: None,
            page_context: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tweet(text: &str, author: Option<&str>) -> TwitterTweet {
        TwitterTweet::new(
            text.to_string(),
            author.map(|a| a.to_string()),
            Some("2024-01-01T00:00:00Z".to_string()),
        )
    }

    #[test]
    fn test_twitter_snapshot_creation() {
        let tweets = vec![
            create_test_tweet("Hello world!", Some("user1")),
            create_test_tweet("Testing #rust", Some("user2")),
        ];

        let snapshot = TwitterSnapshot::new(tweets);

        assert_eq!(snapshot.get_tweet_count(), 2);
        assert!(snapshot.has_tweets());
        assert!(snapshot.created_at > 0);
        assert_eq!(snapshot.created_at, snapshot.updated_at);
        assert_eq!(snapshot.interaction_type, None);
    }

    #[test]
    fn test_snapshot_with_interaction() {
        let tweets = vec![create_test_tweet("Great post!", Some("user1"))];

        let snapshot = TwitterSnapshot::with_interaction(
            tweets,
            TwitterInteractionType::Like,
            Some("@user1".to_string()),
        );

        assert_eq!(
            snapshot.interaction_type,
            Some(TwitterInteractionType::Like)
        );
        assert_eq!(snapshot.interaction_target, Some("@user1".to_string()));
        assert!(snapshot.is_interaction(&TwitterInteractionType::Like));
        assert!(!snapshot.is_interaction(&TwitterInteractionType::Retweet));
    }

    #[test]
    fn test_full_context_snapshot() {
        let tweets = vec![create_test_tweet("Context tweet", Some("user1"))];

        let snapshot = TwitterSnapshot::with_full_context(
            tweets,
            Some(TwitterInteractionType::View),
            None,
            Some(0.5),
            Some("timeline".to_string()),
        );

        assert_eq!(
            snapshot.interaction_type,
            Some(TwitterInteractionType::View)
        );
        assert_eq!(snapshot.scroll_position, Some(0.5));
        assert_eq!(snapshot.page_context, Some("timeline".to_string()));
    }

    #[test]
    fn test_hashtag_extraction() {
        let tweets = vec![
            create_test_tweet("Learning #rust today", Some("user1")),
            create_test_tweet("More #rust and #programming", Some("user2")),
        ];

        let snapshot = TwitterSnapshot::new(tweets);
        let hashtags = snapshot.get_hashtags();

        assert_eq!(hashtags, vec!["#programming", "#rust"]);
    }

    #[test]
    fn test_tweet_search() {
        let tweets = vec![
            create_test_tweet("Learning Rust programming", Some("user1")),
            create_test_tweet("Python is also great", Some("user2")),
        ];

        let snapshot = TwitterSnapshot::new(tweets);

        let rust_tweets = snapshot.search_tweets("rust");
        assert_eq!(rust_tweets.len(), 1);
        assert!(rust_tweets[0].text.contains("Rust"));

        let programming_tweets = snapshot.search_tweets("programming");
        assert_eq!(programming_tweets.len(), 1);
    }

    #[test]
    fn test_tweets_by_author() {
        let tweets = vec![
            create_test_tweet("First tweet", Some("user1")),
            create_test_tweet("Second tweet", Some("user2")),
            create_test_tweet("Third tweet", Some("user1")),
        ];

        let snapshot = TwitterSnapshot::new(tweets);
        let user1_tweets = snapshot.get_tweets_by_author("user1");

        assert_eq!(user1_tweets.len(), 2);
        assert!(user1_tweets[0].text.contains("First"));
        assert!(user1_tweets[1].text.contains("Third"));
    }

    #[test]
    fn test_interaction_description() {
        let tweets = vec![create_test_tweet("Test", Some("user1"))];

        let like_snapshot = TwitterSnapshot::with_interaction(
            tweets.clone(),
            TwitterInteractionType::Like,
            Some("@user1".to_string()),
        );

        assert_eq!(
            like_snapshot.get_interaction_description(),
            Some("Liked @user1".to_string())
        );

        let view_snapshot =
            TwitterSnapshot::with_interaction(tweets, TwitterInteractionType::View, None);

        assert_eq!(
            view_snapshot.get_interaction_description(),
            Some("Viewing".to_string())
        );
    }

    #[test]
    fn test_message_construction() {
        let tweets = vec![
            create_test_tweet("Hello world!", Some("user1")),
            create_test_tweet("Testing #rust", Some("user2")),
        ];

        let snapshot = TwitterSnapshot::with_full_context(
            tweets,
            Some(TwitterInteractionType::Like),
            Some("@user1".to_string()),
            None,
            Some("timeline".to_string()),
        );

        let message = snapshot.construct_message();

        match message.content {
            MessageContent::Text(text) => {
                assert!(text.contains("timeline"));
                assert!(text.contains("liking"));
                assert!(text.contains("@user1"));
                assert!(text.contains("Hello world!"));
                assert!(text.contains("Testing #rust"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_touch_updates_timestamp() {
        let mut snapshot = TwitterSnapshot::new(vec![]);
        let original_updated_at = snapshot.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        snapshot.touch();

        assert!(snapshot.updated_at >= original_updated_at);
    }

    #[test]
    fn test_empty_snapshot() {
        let snapshot = TwitterSnapshot::new(vec![]);

        assert_eq!(snapshot.get_tweet_count(), 0);
        assert!(!snapshot.has_tweets());
        assert!(snapshot.get_hashtags().is_empty());
        assert!(snapshot.search_tweets("anything").is_empty());
    }
}
