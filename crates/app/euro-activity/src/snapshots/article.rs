use agent_chain_core::messages::{ContentBlocks, PlainTextContentBlock};
use euro_native_messaging::types::NativeArticleSnapshot;
use serde::{Deserialize, Serialize};

use crate::{ActivityResult, types::SnapshotFunctionality};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSnapshot {
    pub id: String,
    pub highlighted_text: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl ArticleSnapshot {
    pub fn new(id: Option<String>, highlight: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        Self {
            id,
            highlighted_text: highlight,
            created_at: now,
            updated_at: now,
        }
    }

    fn try_from(snapshot: NativeArticleSnapshot) -> ActivityResult<Self> {
        let now = chrono::Utc::now().timestamp() as u64;
        Ok(ArticleSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            highlighted_text: snapshot.highlighted_text,
            created_at: now,
            updated_at: now,
        })
    }
}

impl SnapshotFunctionality for ArticleSnapshot {
    fn construct_messages(&self) -> ContentBlocks {
        match &self.highlighted_text {
            None => return ContentBlocks::new(),
            Some(h) if h.is_empty() => return ContentBlocks::new(),
            _ => {}
        }

        let snapshot_json = serde_json::to_string(&self).ok();

        match snapshot_json {
            None => ContentBlocks::new(),
            Some(json) => {
                let block = PlainTextContentBlock::builder()
                    .title("article_snapshot.json".to_string())
                    .mime_type("application/json".to_string())
                    .text(json)
                    .build();
                vec![block.into()].into()
            }
        }
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
    use agent_chain_core::messages::ContentBlock;

    #[test]
    fn test_article_snapshot_creation() {
        let snapshot = ArticleSnapshot::new(None, Some("Highlighted text".to_string()));

        assert_eq!(
            snapshot.highlighted_text,
            Some("Highlighted text".to_string())
        );
        assert!(snapshot.created_at > 0);
        assert_eq!(snapshot.created_at, snapshot.updated_at);
    }

    #[test]
    fn test_article_snapshot_with_context() {
        let snapshot = ArticleSnapshot::new(None, Some("Highlighted text".to_string()));

        assert_eq!(
            snapshot.highlighted_text,
            Some("Highlighted text".to_string())
        );
    }

    #[test]
    fn test_message_construction() {
        let snapshot = ArticleSnapshot::new(None, Some("Important quote".to_string()));

        let blocks = snapshot.construct_messages();
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
    }
}
