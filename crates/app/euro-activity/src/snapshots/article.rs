use agent_chain_core::{BaseMessage, HumanMessage};
use euro_native_messaging::types::NativeArticleSnapshot;
use serde::{Deserialize, Serialize};

use crate::{ActivityResult, types::SnapshotFunctionality};

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
}

impl SnapshotFunctionality for ArticleSnapshot {
    fn construct_messages(&self) -> Vec<BaseMessage> {
        if let Some(highlight) = &self.highlight
            && highlight.is_empty()
        {
            return vec![];
        }
        let mut content = String::new();

        if let Some(title) = &self.page_title {
            content.push_str(&format!("From article titled '{}': ", title));
        }

        if let Some(highlight) = &self.highlight {
            content.push_str(&format!(
                "user highlighted the following text: \"{}\"",
                highlight
            ));
        } else if let Some(selection) = &self.selection_text {
            content.push_str(&format!(
                "user selected the following text: \"{}\"",
                selection
            ));
        } else {
            content.push_str("user is reading an article");
        }

        if let Some(url) = &self.page_url {
            content.push_str(&format!(" (from: {})", url));
        }

        vec![HumanMessage::builder().content(content).build().into()]
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
}
