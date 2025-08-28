//! Article snapshot implementation

use eur_proto::ipc::ProtoArticleSnapshot;
use ferrous_llm_core::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};

/// Article snapshot capturing highlights and reading progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSnapshot {
    pub highlight: Option<String>,
    pub selected_text: Option<String>,
    pub scroll_position: Option<f32>,  // 0.0 to 1.0
    pub reading_progress: Option<f32>, // 0.0 to 1.0
    pub page_url: Option<String>,
    pub page_title: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl ArticleSnapshot {
    /// Create a new article snapshot
    pub fn new(
        highlight: Option<String>,
        selected_text: Option<String>,
        scroll_position: Option<f32>,
        reading_progress: Option<f32>,
        page_url: Option<String>,
        page_title: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            highlight,
            selected_text,
            scroll_position,
            reading_progress,
            page_url,
            page_title,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a simple highlight snapshot
    pub fn highlight(text: String) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            highlight: Some(text),
            selected_text: None,
            scroll_position: None,
            reading_progress: None,
            page_url: None,
            page_title: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a reading progress snapshot
    pub fn progress(progress: f32, scroll_position: Option<f32>) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            highlight: None,
            selected_text: None,
            scroll_position,
            reading_progress: Some(progress.clamp(0.0, 1.0)),
            page_url: None,
            page_title: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Try to create from protocol buffer snapshot
    pub fn try_from(snapshot: ProtoArticleSnapshot) -> Result<Self, crate::error::ActivityError> {
        let now = chrono::Utc::now().timestamp() as u64;
        Ok(ArticleSnapshot {
            highlight: if snapshot.highlighted_content.is_empty() {
                None
            } else {
                Some(snapshot.highlighted_content)
            },
            selected_text: None,
            scroll_position: None,
            reading_progress: None,
            page_url: None,
            page_title: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Construct a message for LLM interaction
    pub fn construct_message(&self) -> Message {
        let mut content = String::new();

        if let Some(highlight) = &self.highlight {
            content.push_str(&format!(
                "I highlighted the following text: \"{}\"",
                highlight
            ));
        } else if let Some(selected) = &self.selected_text {
            content.push_str(&format!("I selected the following text: \"{}\"", selected));
        } else if let Some(progress) = self.reading_progress {
            content.push_str(&format!(
                "I am {}% through reading this article",
                (progress * 100.0) as u32
            ));
        } else {
            content.push_str("I am reading an article");
        }

        if let Some(title) = &self.page_title {
            content.push_str(&format!(" titled \"{}\"", title));
        }

        if let Some(scroll) = self.scroll_position {
            content.push_str(&format!(
                " (currently at {}% of the page)",
                (scroll * 100.0) as u32
            ));
        }

        content.push('.');

        Message {
            role: Role::User,
            content: MessageContent::Text(content),
        }
    }

    /// Check if this snapshot contains a highlight
    pub fn has_highlight(&self) -> bool {
        self.highlight.is_some()
    }

    /// Check if this snapshot contains selected text
    pub fn has_selection(&self) -> bool {
        self.selected_text.is_some()
    }

    /// Check if this snapshot contains reading progress
    pub fn has_progress(&self) -> bool {
        self.reading_progress.is_some()
    }

    /// Get the main content (highlight, selection, or empty)
    pub fn get_main_content(&self) -> Option<&str> {
        self.highlight
            .as_deref()
            .or_else(|| self.selected_text.as_deref())
    }

    /// Get reading progress as percentage (0-100)
    pub fn get_progress_percentage(&self) -> Option<u32> {
        self.reading_progress.map(|p| (p * 100.0) as u32)
    }

    /// Get scroll position as percentage (0-100)
    pub fn get_scroll_percentage(&self) -> Option<u32> {
        self.scroll_position.map(|p| (p * 100.0) as u32)
    }

    /// Check if the user is near the end of the article
    pub fn is_near_end(&self) -> bool {
        self.reading_progress.map_or(false, |p| p >= 0.9)
            || self.scroll_position.map_or(false, |p| p >= 0.9)
    }

    /// Check if the user just started reading
    pub fn is_at_beginning(&self) -> bool {
        self.reading_progress.map_or(false, |p| p <= 0.1)
            && self.scroll_position.map_or(true, |p| p <= 0.1)
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp() as u64;
    }

    /// Merge with another snapshot, keeping the most recent data
    pub fn merge_with(&mut self, other: &ArticleSnapshot) {
        if other.updated_at > self.updated_at {
            if other.highlight.is_some() {
                self.highlight = other.highlight.clone();
            }
            if other.selected_text.is_some() {
                self.selected_text = other.selected_text.clone();
            }
            if other.scroll_position.is_some() {
                self.scroll_position = other.scroll_position;
            }
            if other.reading_progress.is_some() {
                self.reading_progress = other.reading_progress;
            }
            if other.page_url.is_some() {
                self.page_url = other.page_url.clone();
            }
            if other.page_title.is_some() {
                self.page_title = other.page_title.clone();
            }
            self.updated_at = other.updated_at;
        }
    }
}

impl From<ProtoArticleSnapshot> for ArticleSnapshot {
    fn from(snapshot: ProtoArticleSnapshot) -> Self {
        Self::try_from(snapshot).expect("Failed to convert ProtoArticleSnapshot to ArticleSnapshot")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_snapshot_creation() {
        let snapshot = ArticleSnapshot::new(
            Some("Important text".to_string()),
            None,
            Some(0.5),
            Some(0.3),
            Some("https://example.com/article".to_string()),
            Some("Test Article".to_string()),
        );

        assert_eq!(snapshot.highlight, Some("Important text".to_string()));
        assert_eq!(snapshot.scroll_position, Some(0.5));
        assert_eq!(snapshot.reading_progress, Some(0.3));
        assert!(snapshot.created_at > 0);
        assert_eq!(snapshot.created_at, snapshot.updated_at);
    }

    #[test]
    fn test_highlight_snapshot() {
        let snapshot = ArticleSnapshot::highlight("This is highlighted".to_string());

        assert_eq!(snapshot.highlight, Some("This is highlighted".to_string()));
        assert!(snapshot.has_highlight());
        assert!(!snapshot.has_selection());
        assert!(!snapshot.has_progress());
    }

    #[test]
    fn test_progress_snapshot() {
        let snapshot = ArticleSnapshot::progress(0.75, Some(0.8));

        assert_eq!(snapshot.reading_progress, Some(0.75));
        assert_eq!(snapshot.scroll_position, Some(0.8));
        assert!(snapshot.has_progress());
        assert!(!snapshot.has_highlight());
    }

    #[test]
    fn test_progress_clamping() {
        let snapshot = ArticleSnapshot::progress(1.5, None); // Should clamp to 1.0
        assert_eq!(snapshot.reading_progress, Some(1.0));

        let snapshot2 = ArticleSnapshot::progress(-0.1, None); // Should clamp to 0.0
        assert_eq!(snapshot2.reading_progress, Some(0.0));
    }

    #[test]
    fn test_main_content() {
        let highlight_snapshot = ArticleSnapshot::highlight("Highlighted text".to_string());
        assert_eq!(
            highlight_snapshot.get_main_content(),
            Some("Highlighted text")
        );

        let selection_snapshot = ArticleSnapshot::new(
            None,
            Some("Selected text".to_string()),
            None,
            None,
            None,
            None,
        );
        assert_eq!(selection_snapshot.get_main_content(), Some("Selected text"));

        let empty_snapshot = ArticleSnapshot::progress(0.5, None);
        assert_eq!(empty_snapshot.get_main_content(), None);
    }

    #[test]
    fn test_percentage_calculations() {
        let snapshot = ArticleSnapshot::new(None, None, Some(0.75), Some(0.6), None, None);

        assert_eq!(snapshot.get_progress_percentage(), Some(60));
        assert_eq!(snapshot.get_scroll_percentage(), Some(75));
    }

    #[test]
    fn test_position_detection() {
        let near_end = ArticleSnapshot::progress(0.95, Some(0.9));
        assert!(near_end.is_near_end());
        assert!(!near_end.is_at_beginning());

        let at_beginning = ArticleSnapshot::progress(0.05, Some(0.05));
        assert!(at_beginning.is_at_beginning());
        assert!(!at_beginning.is_near_end());

        let middle = ArticleSnapshot::progress(0.5, Some(0.5));
        assert!(!middle.is_at_beginning());
        assert!(!middle.is_near_end());
    }

    #[test]
    fn test_merge_snapshots() {
        let mut older = ArticleSnapshot::new(
            Some("Old highlight".to_string()),
            None,
            Some(0.3),
            Some(0.2),
            None,
            None,
        );

        // Sleep to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(1));

        let newer = ArticleSnapshot::new(
            Some("New highlight".to_string()),
            Some("New selection".to_string()),
            Some(0.7),
            Some(0.6),
            Some("https://example.com".to_string()),
            Some("New Title".to_string()),
        );

        older.merge_with(&newer);

        assert_eq!(older.highlight, Some("New highlight".to_string()));
        assert_eq!(older.selected_text, Some("New selection".to_string()));
        assert_eq!(older.scroll_position, Some(0.7));
        assert_eq!(older.reading_progress, Some(0.6));
        assert_eq!(older.page_url, Some("https://example.com".to_string()));
        assert_eq!(older.page_title, Some("New Title".to_string()));
    }

    #[test]
    fn test_message_construction() {
        let highlight_snapshot = ArticleSnapshot::new(
            Some("Important quote".to_string()),
            None,
            Some(0.5),
            None,
            None,
            Some("Test Article".to_string()),
        );

        let message = highlight_snapshot.construct_message();

        match message.content {
            MessageContent::Text(text) => {
                assert!(text.contains("Important quote"));
                assert!(text.contains("Test Article"));
                assert!(text.contains("50%"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_touch_updates_timestamp() {
        let mut snapshot = ArticleSnapshot::highlight("Test".to_string());
        let original_updated_at = snapshot.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        snapshot.touch();

        assert!(snapshot.updated_at >= original_updated_at);
    }
}
