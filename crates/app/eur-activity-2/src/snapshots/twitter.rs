//! Twitter snapshot implementation

use crate::assets::twitter::TwitterTweet;
use eur_proto::ipc::{ProtoTweet, ProtoTwitterSnapshot};
use ferrous_llm_core::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};

/// Twitter snapshot capturing real-time tweet updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterSnapshot {
    pub tweets: Vec<TwitterTweet>,
    pub context_type: TwitterContextType,
    pub scroll_position: Option<f32>, // 0.0 to 1.0
    pub active_tweet_id: Option<String>,
    pub interaction_type: Option<TwitterInteractionType>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Type of Twitter context being captured
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TwitterContextType {
    Timeline,
    Profile,
    Thread,
    Search,
    Hashtag,
    Notifications,
    DirectMessages,
}

/// Type of interaction with Twitter content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TwitterInteractionType {
    Viewing,
    Liking,
    Retweeting,
    Replying,
    Scrolling,
    Searching,
}

impl TwitterSnapshot {
    /// Create a new Twitter snapshot
    pub fn new(
        tweets: Vec<TwitterTweet>,
        context_type: TwitterContextType,
        scroll_position: Option<f32>,
        active_tweet_id: Option<String>,
        interaction_type: Option<TwitterInteractionType>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            tweets,
            context_type,
            scroll_position,
            active_tweet_id,
            interaction_type,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a simple viewing snapshot
    pub fn viewing(tweets: Vec<TwitterTweet>, context_type: TwitterContextType) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            tweets,
            context_type,
            scroll_position: None,
            active_tweet_id: None,
            interaction_type: Some(TwitterInteractionType::Viewing),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a scrolling snapshot
    pub fn scrolling(
        tweets: Vec<TwitterTweet>,
        context_type: TwitterContextType,
        scroll_position: f32,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            tweets,
            context_type,
            scroll_position: Some(scroll_position.clamp(0.0, 1.0)),
            active_tweet_id: None,
            interaction_type: Some(TwitterInteractionType::Scrolling),
            created_at: now,
            updated_at: now,
        }
    }

    /// Try to create from protocol buffer snapshot
    pub fn try_from(snapshot: ProtoTwitterSnapshot) -> Result<Self, crate::error::ActivityError> {
        let tweets: Vec<TwitterTweet> = snapshot
            .tweets
            .into_iter()
            .map(|tweet| TwitterTweet::from(tweet))
            .collect();

        let now = chrono::Utc::now().timestamp() as u64;
        Ok(TwitterSnapshot {
            tweets,
            context_type: TwitterContextType::Timeline,
            scroll_position: None,
            active_tweet_id: None,
            interaction_type: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Construct a message for LLM interaction
    pub fn construct_message(&self) -> Message {
        let tweet_texts: Vec<String> = self
            .tweets
            .iter()
            .map(|tweet| tweet.get_formatted_text())
            .collect();

        let context_description = match self.context_type {
            TwitterContextType::Timeline => "timeline",
            TwitterContextType::Profile => "profile",
            TwitterContextType::Thread => "thread",
            TwitterContextType::Search => "search results",
            TwitterContextType::Hashtag => "hashtag feed",
            TwitterContextType::Notifications => "notifications",
            TwitterContextType::DirectMessages => "direct messages",
        };

        let interaction_description = match &self.interaction_type {
            Some(TwitterInteractionType::Viewing) => "viewing",
            Some(TwitterInteractionType::Liking) => "liking content in",
            Some(TwitterInteractionType::Retweeting) => "retweeting content from",
            Some(TwitterInteractionType::Replying) => "replying to content in",
            Some(TwitterInteractionType::Scrolling) => "scrolling through",
            Some(TwitterInteractionType::Searching) => "searching in",
            None => "looking at",
        };

        let mut content = format!(
            "I am {} Twitter {} content and have a question about it.",
            interaction_description, context_description
        );

        if let Some(scroll) = self.scroll_position {
            content.push_str(&format!(
                " I'm currently at {}% of the feed.",
                (scroll * 100.0) as u32
            ));
        }

        if !tweet_texts.is_empty() {
            content.push_str(&format!(
                " Here are the tweets I'm seeing: \n\n{}",
                tweet_texts.join("\n\n")
            ));
        }

        Message {
            role: Role::User,
            content: MessageContent::Text(content),
        }
    }

    /// Get the number of tweets in this snapshot
    pub fn get_tweet_count(&self) -> usize {
        self.tweets.len()
    }

    /// Get all unique hashtags from all tweets
    pub fn get_all_hashtags(&self) -> Vec<String> {
        let mut hashtags = Vec::new();
        for tweet in &self.tweets {
            hashtags.extend(tweet.extract_hashtags());
        }
        hashtags.sort();
        hashtags.dedup();
        hashtags
    }

    /// Get all unique mentions from all tweets
    pub fn get_all_mentions(&self) -> Vec<String> {
        let mut mentions = Vec::new();
        for tweet in &self.tweets {
            mentions.extend(tweet.extract_mentions());
        }
        mentions.sort();
        mentions.dedup();
        mentions
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

    /// Search tweets containing specific text
    pub fn search_tweets(&self, query: &str) -> Vec<&TwitterTweet> {
        let query_lower = query.to_lowercase();
        self.tweets
            .iter()
            .filter(|tweet| tweet.text.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get the active tweet if specified
    pub fn get_active_tweet(&self) -> Option<&TwitterTweet> {
        if let Some(active_id) = &self.active_tweet_id {
            // For now, we don't have tweet IDs in TwitterTweet, so we can't match
            // This would need to be implemented when TwitterTweet gets an id field
            None
        } else {
            None
        }
    }

    /// Check if this snapshot represents an interaction (not just viewing)
    pub fn is_interactive(&self) -> bool {
        matches!(
            self.interaction_type,
            Some(TwitterInteractionType::Liking)
                | Some(TwitterInteractionType::Retweeting)
                | Some(TwitterInteractionType::Replying)
                | Some(TwitterInteractionType::Searching)
        )
    }

    /// Get scroll position as percentage (0-100)
    pub fn get_scroll_percentage(&self) -> Option<u32> {
        self.scroll_position.map(|p| (p * 100.0) as u32)
    }

    /// Check if user is at the top of the feed
    pub fn is_at_top(&self) -> bool {
        self.scroll_position.map_or(true, |p| p <= 0.1)
    }

    /// Check if user is near the bottom of the feed
    pub fn is_near_bottom(&self) -> bool {
        self.scroll_position.map_or(false, |p| p >= 0.9)
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp() as u64;
    }

    /// Add new tweets to the snapshot
    pub fn add_tweets(&mut self, mut new_tweets: Vec<TwitterTweet>) {
        self.tweets.append(&mut new_tweets);
        self.touch();
    }

    /// Update scroll position
    pub fn update_scroll_position(&mut self, position: f32) {
        self.scroll_position = Some(position.clamp(0.0, 1.0));
        self.touch();
    }

    /// Set active tweet
    pub fn set_active_tweet(&mut self, tweet_id: String) {
        self.active_tweet_id = Some(tweet_id);
        self.touch();
    }

    /// Update interaction type
    pub fn set_interaction_type(&mut self, interaction_type: TwitterInteractionType) {
        self.interaction_type = Some(interaction_type);
        self.touch();
    }
}

impl From<ProtoTwitterSnapshot> for TwitterSnapshot {
    fn from(snapshot: ProtoTwitterSnapshot) -> Self {
        Self::try_from(snapshot).expect("Failed to convert ProtoTwitterSnapshot to TwitterSnapshot")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tweet(text: &str, author: Option<&str>) -> TwitterTweet {
        TwitterTweet::new(text.to_string(), author.map(|a| a.to_string()), None)
    }

    #[test]
    fn test_twitter_snapshot_creation() {
        let tweets = vec![
            create_test_tweet("Hello world!", Some("user1")),
            create_test_tweet("Another tweet", Some("user2")),
        ];

        let snapshot = TwitterSnapshot::new(
            tweets,
            TwitterContextType::Timeline,
            Some(0.5),
            Some("tweet123".to_string()),
            Some(TwitterInteractionType::Viewing),
        );

        assert_eq!(snapshot.get_tweet_count(), 2);
        assert_eq!(snapshot.scroll_position, Some(0.5));
        assert_eq!(snapshot.active_tweet_id, Some("tweet123".to_string()));
        assert!(snapshot.created_at > 0);
    }

    #[test]
    fn test_viewing_snapshot() {
        let tweets = vec![create_test_tweet("Test tweet", Some("user1"))];
        let snapshot = TwitterSnapshot::viewing(tweets, TwitterContextType::Profile);

        assert_eq!(snapshot.get_tweet_count(), 1);
        assert_eq!(
            snapshot.interaction_type,
            Some(TwitterInteractionType::Viewing)
        );
        assert!(!snapshot.is_interactive());
    }

    #[test]
    fn test_scrolling_snapshot() {
        let tweets = vec![create_test_tweet("Test tweet", Some("user1"))];
        let snapshot = TwitterSnapshot::scrolling(tweets, TwitterContextType::Timeline, 0.75);

        assert_eq!(snapshot.scroll_position, Some(0.75));
        assert_eq!(
            snapshot.interaction_type,
            Some(TwitterInteractionType::Scrolling)
        );
        assert!(!snapshot.is_interactive());
    }

    #[test]
    fn test_scroll_clamping() {
        let tweets = vec![create_test_tweet("Test tweet", Some("user1"))];
        let snapshot = TwitterSnapshot::scrolling(tweets, TwitterContextType::Timeline, 1.5);

        assert_eq!(snapshot.scroll_position, Some(1.0)); // Should clamp to 1.0
    }

    #[test]
    fn test_hashtag_extraction() {
        let tweets = vec![
            create_test_tweet("Learning #rust today! #programming", Some("user1")),
            create_test_tweet("More #rust content", Some("user2")),
        ];

        let snapshot = TwitterSnapshot::viewing(tweets, TwitterContextType::Timeline);
        let hashtags = snapshot.get_all_hashtags();

        assert_eq!(hashtags, vec!["#programming", "#rust"]);
    }

    #[test]
    fn test_tweet_search() {
        let tweets = vec![
            create_test_tweet("Learning Rust programming", Some("user1")),
            create_test_tweet("Python is also great", Some("user2")),
            create_test_tweet("More Rust content", Some("user3")),
        ];

        let snapshot = TwitterSnapshot::viewing(tweets, TwitterContextType::Timeline);
        let rust_tweets = snapshot.search_tweets("rust");

        assert_eq!(rust_tweets.len(), 2);
        assert!(rust_tweets[0].text.to_lowercase().contains("rust"));
        assert!(rust_tweets[1].text.to_lowercase().contains("rust"));
    }

    #[test]
    fn test_tweets_by_author() {
        let tweets = vec![
            create_test_tweet("First tweet", Some("user1")),
            create_test_tweet("Second tweet", Some("user2")),
            create_test_tweet("Third tweet", Some("user1")),
        ];

        let snapshot = TwitterSnapshot::viewing(tweets, TwitterContextType::Timeline);
        let user1_tweets = snapshot.get_tweets_by_author("user1");

        assert_eq!(user1_tweets.len(), 2);
        assert_eq!(user1_tweets[0].text, "First tweet");
        assert_eq!(user1_tweets[1].text, "Third tweet");
    }

    #[test]
    fn test_position_detection() {
        let tweets = vec![create_test_tweet("Test", Some("user1"))];

        let top_snapshot =
            TwitterSnapshot::scrolling(tweets.clone(), TwitterContextType::Timeline, 0.05);
        assert!(top_snapshot.is_at_top());
        assert!(!top_snapshot.is_near_bottom());

        let bottom_snapshot =
            TwitterSnapshot::scrolling(tweets, TwitterContextType::Timeline, 0.95);
        assert!(!bottom_snapshot.is_at_top());
        assert!(bottom_snapshot.is_near_bottom());
    }

    #[test]
    fn test_interactive_detection() {
        let tweets = vec![create_test_tweet("Test", Some("user1"))];

        let viewing = TwitterSnapshot::new(
            tweets.clone(),
            TwitterContextType::Timeline,
            None,
            None,
            Some(TwitterInteractionType::Viewing),
        );
        assert!(!viewing.is_interactive());

        let liking = TwitterSnapshot::new(
            tweets,
            TwitterContextType::Timeline,
            None,
            None,
            Some(TwitterInteractionType::Liking),
        );
        assert!(liking.is_interactive());
    }

    #[test]
    fn test_snapshot_updates() {
        let mut snapshot = TwitterSnapshot::viewing(
            vec![create_test_tweet("Original", Some("user1"))],
            TwitterContextType::Timeline,
        );

        let original_updated_at = snapshot.updated_at;

        // Sleep to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        snapshot.add_tweets(vec![create_test_tweet("New tweet", Some("user2"))]);
        assert_eq!(snapshot.get_tweet_count(), 2);
        assert!(snapshot.updated_at >= original_updated_at);

        snapshot.update_scroll_position(0.8);
        assert_eq!(snapshot.scroll_position, Some(0.8));

        snapshot.set_active_tweet("tweet456".to_string());
        assert_eq!(snapshot.active_tweet_id, Some("tweet456".to_string()));

        snapshot.set_interaction_type(TwitterInteractionType::Liking);
        assert_eq!(
            snapshot.interaction_type,
            Some(TwitterInteractionType::Liking)
        );
    }

    #[test]
    fn test_message_construction() {
        let tweets = vec![
            create_test_tweet("First tweet", Some("user1")),
            create_test_tweet("Second tweet", Some("user2")),
        ];

        let snapshot = TwitterSnapshot::new(
            tweets,
            TwitterContextType::Timeline,
            Some(0.6),
            None,
            Some(TwitterInteractionType::Scrolling),
        );

        let message = snapshot.construct_message();

        match message.content {
            MessageContent::Text(text) => {
                assert!(text.contains("scrolling"));
                assert!(text.contains("timeline"));
                assert!(text.contains("60%"));
                assert!(text.contains("First tweet"));
                assert!(text.contains("Second tweet"));
                assert!(text.contains("@user1"));
                assert!(text.contains("@user2"));
            }
            _ => panic!("Expected text content"),
        }
    }
}
