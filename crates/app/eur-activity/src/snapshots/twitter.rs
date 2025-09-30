//! Twitter snapshot implementation

use eur_native_messaging::types::NativeTwitterSnapshot;
use ferrous_llm_core::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ActivityResult, assets::twitter::TwitterTweet, types::SnapshotFunctionality};

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
    pub id: String,
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
    pub fn new(
        id: Option<String>,
        tweets: Vec<TwitterTweet>,
        interaction_type: TwitterInteractionType,
        interaction_target: Option<String>,
        scroll_position: Option<f32>,
        page_context: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        Self {
            id,
            tweets,
            interaction_type: Some(interaction_type),
            interaction_target,
            scroll_position,
            page_context,
            created_at: now,
            updated_at: now,
        }
    }

    fn try_from(snapshot: NativeTwitterSnapshot) -> ActivityResult<Self> {
        let tweets: Vec<TwitterTweet> = snapshot
            .tweets
            .into_iter()
            .map(TwitterTweet::from)
            .collect();

        let now = chrono::Utc::now().timestamp() as u64;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            tweets,
            interaction_type: None,
            interaction_target: None,
            scroll_position: None,
            page_context: None,
            created_at: now,
            updated_at: now,
        })
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
    fn construct_messages(&self) -> Vec<Message> {
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

        vec![Message {
            role: Role::User,
            content: MessageContent::Text(content),
        }]
    }

    fn get_updated_at(&self) -> u64 {
        self.updated_at
    }

    fn get_created_at(&self) -> u64 {
        self.created_at
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

impl From<NativeTwitterSnapshot> for TwitterSnapshot {
    fn from(snapshot: NativeTwitterSnapshot) -> Self {
        Self::try_from(snapshot)
            .expect("Failed to convert NativeTwitterSnapshot to TwitterSnapshot")
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
}
