//! Message tree wire types and the search shapes that operate on it.

use agent_chain_core::messages::AnyMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

/// One node in the message tree returned by message-list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct MessageNode {
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    pub message: AnyMessage,
    #[serde(default)]
    pub children: Vec<MessageNode>,
    pub sibling_index: i32,
    pub depth: i32,
}

/// Query parameters for `GET /threads/{thread_id}/messages`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GetMessagesQuery {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for endpoints that return the message tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GetMessagesResponse {
    pub messages: Vec<MessageNode>,
}

/// Request body for `POST /threads/{thread_id}/messages/switch-branch`.
///
/// `direction` is `-1`, `0`, or `1` — left sibling, no sibling change, right
/// sibling — matching the gRPC contract this replaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SwitchBranchRequest {
    pub message_id: Uuid,
    pub direction: i32,
}

/// Query parameters for `GET /threads/messages/search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchMessagesQuery {
    pub q: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for `GET /threads/messages/search`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchMessagesResponse {
    pub results: Vec<SearchMessageResult>,
}

/// One message hit returned by full-text search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchMessageResult {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub message_type: String,
    pub rank: f32,
    pub created_at: DateTime<Utc>,
    pub snippet: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_chain_core::messages::HumanMessage;

    fn sample_human_message() -> AnyMessage {
        AnyMessage::HumanMessage(HumanMessage::builder().content("hi").build())
    }

    #[test]
    fn message_node_round_trips() {
        let node = MessageNode {
            parent_id: Some(Uuid::nil()),
            message: sample_human_message(),
            children: vec![],
            sibling_index: 0,
            depth: 0,
        };
        let s = serde_json::to_string(&node).unwrap();
        let back: MessageNode = serde_json::from_str(&s).unwrap();
        assert_eq!(node, back);
    }
}
