//! Article snapshot implementation

use agent_chain::{BaseMessage, HumanMessage};
use euro_native_messaging::types::NativeArticleSnapshot;
use serde::{Deserialize, Serialize};

use crate::{ActivityResult, types::SnapshotFunctionality};

/// Article snapshot with highlighted content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSnapshot {
    pub id: String,
    pub highlight: Option<String>,
    pub selection_text: Option<String>,
    pub page_url: Option<String>,
    pub page_title: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl ArticleSnapshot {
    /// Create a new article snapshot
    pub fn new(
        id: Option<String>,
        highlight: Option<String>,
        selection_text: Option<String>,
        page_url: Option<String>,
        page_title: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        Self {
            id,
            highlight,
            selection_text,
            page_url,
            page_title,
            created_at: now,
            updated_at: now,
        }
    }

    fn try_from(snapshot: NativeArticleSnapshot) -> ActivityResult<Self> {
        let now = chrono::Utc::now().timestamp() as u64;
        Ok(ArticleSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            highlight: snapshot.highlighted_text,
            selection_text: None,
            page_url: None,
            page_title: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp() as u64;
    }

    /// Check if the snapshot has any content
    pub fn has_content(&self) -> bool {
        self.highlight.is_some() || self.selection_text.is_some()
    }

    /// Get the primary content (highlight or selection)
    pub fn get_primary_content(&self) -> Option<&str> {
        self.highlight.as_deref().or(self.selection_text.as_deref())
    }

    /// Get content length
    pub fn get_content_length(&self) -> usize {
        self.get_primary_content()
            .map_or(0, |content| content.len())
    }

    /// Check if content contains a keyword
    pub fn contains_keyword(&self, keyword: &str) -> bool {
        let keyword_lower = keyword.to_lowercase();

        if let Some(content) = self.get_primary_content()
            && content.to_lowercase().contains(&keyword_lower)
        {
            return true;
        }

        if let Some(title) = &self.page_title
            && title.to_lowercase().contains(&keyword_lower)
        {
            return true;
        }

        false
    }
}

impl SnapshotFunctionality for ArticleSnapshot {
    /// Construct a message for LLM interaction
    fn construct_messages(&self) -> Vec<BaseMessage> {
        let mut content = String::new();

        if let Some(title) = &self.page_title {
            content.push_str(&format!("From article titled '{}': ", title));
        }

        if let Some(highlight) = &self.highlight {
            content.push_str(&format!(
                "I highlighted the following text: \"{}\"",
                highlight
            ));
        } else if let Some(selection) = &self.selection_text {
            content.push_str(&format!("I selected the following text: \"{}\"", selection));
        } else {
            content.push_str("I'm reading an article");
        }

        if let Some(url) = &self.page_url {
            content.push_str(&format!(" (from: {})", url));
        }

        vec![HumanMessage::new(content).into()]
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

impl From<NativeArticleSnapshot> for ArticleSnapshot {
    fn from(snapshot: NativeArticleSnapshot) -> Self {
        Self::try_from(snapshot)
            .expect("Failed to convert NativeArticleSnapshot to ArticleSnapshot")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_snapshot_creation() {
        let snapshot = ArticleSnapshot::new(
            None,
            Some("Highlighted text".to_string()),
            Some("Selected text".to_string()),
            Some("https://example.com/article".to_string()),
            Some("Test Article".to_string()),
        );

        assert_eq!(snapshot.highlight, Some("Highlighted text".to_string()));
        assert!(snapshot.created_at > 0);
        assert_eq!(snapshot.created_at, snapshot.updated_at);
        assert!(snapshot.has_content());
    }

    #[test]
    fn test_article_snapshot_with_context() {
        let snapshot = ArticleSnapshot::new(
            None,
            Some("Highlighted text".to_string()),
            Some("Selected text".to_string()),
            Some("https://example.com/article".to_string()),
            Some("Test Article".to_string()),
        );

        assert_eq!(snapshot.highlight, Some("Highlighted text".to_string()));
        assert_eq!(snapshot.selection_text, Some("Selected text".to_string()));
        assert_eq!(
            snapshot.page_url,
            Some("https://example.com/article".to_string())
        );
        assert_eq!(snapshot.page_title, Some("Test Article".to_string()));
    }

    #[test]
    fn test_primary_content() {
        let with_highlight = ArticleSnapshot::new(
            None,
            Some("Highlighted text".to_string()),
            Some("Selected text".to_string()),
            Some("https://example.com/article".to_string()),
            Some("Test Article".to_string()),
        );
        assert_eq!(
            with_highlight.get_primary_content(),
            Some("Highlighted text")
        );

        let with_selection =
            ArticleSnapshot::new(None, None, Some("Selection".to_string()), None, None);
        assert_eq!(with_selection.get_primary_content(), Some("Selection"));

        let empty = ArticleSnapshot::new(None, None, None, None, None);
        assert_eq!(empty.get_primary_content(), None);
        assert!(!empty.has_content());
    }

    #[test]
    fn test_content_length() {
        let snapshot =
            ArticleSnapshot::new(None, Some("Hello world".to_string()), None, None, None);
        assert_eq!(snapshot.get_content_length(), 11);

        let empty = ArticleSnapshot::new(None, None, None, None, None);
        assert_eq!(empty.get_content_length(), 0);
    }

    #[test]
    fn test_keyword_search() {
        let snapshot = ArticleSnapshot::new(
            None,
            Some("Rust programming language".to_string()),
            None,
            None,
            Some("Learning Rust".to_string()),
        );

        assert!(snapshot.contains_keyword("rust"));
        assert!(snapshot.contains_keyword("Rust"));
        assert!(snapshot.contains_keyword("programming"));
        assert!(snapshot.contains_keyword("learning"));
        assert!(!snapshot.contains_keyword("python"));
    }

    #[test]
    fn test_message_construction() {
        let snapshot = ArticleSnapshot::new(
            None,
            Some("Important quote".to_string()),
            None,
            Some("https://example.com".to_string()),
            Some("Test Article".to_string()),
        );

        let message = snapshot.construct_messages()[0].clone();
        let text = message.content();

        assert!(text.contains("Test Article"));
        assert!(text.contains("Important quote"));
        assert!(text.contains("https://example.com"));
    }

    #[test]
    fn test_touch_updates_timestamp() {
        let mut snapshot = ArticleSnapshot::new(None, Some("Test".to_string()), None, None, None);
        let original_updated_at = snapshot.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        snapshot.touch();

        assert!(snapshot.updated_at >= original_updated_at);
    }
}
