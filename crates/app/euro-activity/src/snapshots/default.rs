use agent_chain_core::messages::{ContentBlocks, PlainTextContentBlock};
use serde::{Deserialize, Serialize};

use crate::types::SnapshotFunctionality;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSnapshot {
    pub id: String,
    pub state: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl DefaultSnapshot {
    pub fn new(state: String) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            state,
            metadata: std::collections::HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl SnapshotFunctionality for DefaultSnapshot {
    fn construct_messages(&self) -> ContentBlocks {
        let snapshot_json = serde_json::to_string(&self).unwrap_or_default();

        let block = PlainTextContentBlock::builder()
            .context(format!("Current application state: {}", self.state))
            .title("default_snapshot.json".to_string())
            .mime_type("application/json".to_string())
            .text(snapshot_json)
            .build();

        vec![block.into()].into()
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

#[cfg(test)]
mod tests {
    use super::*;
    use agent_chain_core::messages::ContentBlock;

    #[test]
    fn test_default_snapshot_creation() {
        let snapshot = DefaultSnapshot::new("Active window: Notepad".to_string());

        assert_eq!(snapshot.state, "Active window: Notepad");
        assert!(snapshot.metadata.is_empty());
        assert!(snapshot.created_at > 0);
        assert_eq!(snapshot.created_at, snapshot.updated_at);
    }

    #[test]
    fn test_message_construction_no_metadata() {
        let snapshot = DefaultSnapshot::new("Simple state".to_string());
        let blocks = snapshot.construct_messages();
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
    }
}
