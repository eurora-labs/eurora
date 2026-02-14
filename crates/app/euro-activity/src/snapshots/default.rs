use agent_chain_core::{BaseMessage, HumanMessage};
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
    fn construct_messages(&self) -> Vec<BaseMessage> {
        let mut content = format!("Current application state: {}", self.state);

        if !self.metadata.is_empty() {
            content.push_str(" with additional context:");
            for (key, value) in &self.metadata {
                content.push_str(&format!("\n- {}: {}", key, value));
            }
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let message = snapshot.construct_messages()[0].clone();
        let text = message.content();

        assert_eq!(text, "Current application state: Simple state");
    }
}
