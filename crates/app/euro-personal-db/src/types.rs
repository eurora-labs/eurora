//! Database types for euro-personal-db.
//!
//! These types are designed to closely match the agent-chain-core message types
//! for minimal conversion overhead when saving/loading conversations.

use agent_chain_core::{
    AIMessage, BaseMessage, HumanMessage, MessageContent, SystemMessage, ToolCall, ToolMessage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::Error as DeError};
use sqlx::FromRow;

#[cfg(feature = "specta")]
use specta::Type;

/// Message type discriminator matching agent-chain-core's BaseMessage variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Human,
    System,
    #[serde(rename = "ai")]
    AI,
    Tool,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Human => "human",
            MessageType::System => "system",
            MessageType::AI => "ai",
            MessageType::Tool => "tool",
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for MessageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(MessageType::Human),
            "system" => Ok(MessageType::System),
            "ai" => Ok(MessageType::AI),
            "tool" => Ok(MessageType::Tool),
            _ => Err(format!("Unknown message type: {}", s)),
        }
    }
}

/// Database representation of a conversation.
#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Conversation {
    pub id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database representation of a message.
///
/// This type stores agent-chain messages in a normalized form suitable for SQLite.
/// The content and tool_calls fields are stored as JSON.
#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub message_type: String,
    /// JSON-encoded content (MessageContent for human, String for others)
    pub content: String,
    /// For ToolMessage: the ID of the tool call this responds to
    pub tool_call_id: Option<String>,
    /// For AIMessage: JSON array of ToolCall objects
    pub tool_calls: Option<String>,
    /// Additional metadata as JSON
    pub additional_kwargs: String,
    pub sequence_num: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Message {
    /// Convert this database message to an agent-chain BaseMessage.
    ///
    /// This is the primary way to reconstruct agent-chain messages from the database.
    pub fn to_base_message(&self) -> Result<BaseMessage, serde_json::Error> {
        let message_type: MessageType = self
            .message_type
            .parse()
            .map_err(|e: String| serde_json::Error::custom(e))?;

        match message_type {
            MessageType::Human => {
                let content: MessageContent = serde_json::from_str(&self.content)?;
                let msg = match content {
                    MessageContent::Text(text) => HumanMessage::with_id(&self.id, text),
                    MessageContent::Parts(parts) => {
                        HumanMessage::with_id_and_content(&self.id, parts)
                    }
                };
                Ok(BaseMessage::Human(msg))
            }
            MessageType::System => {
                let content: String = serde_json::from_str(&self.content)?;
                Ok(BaseMessage::System(SystemMessage::with_id(
                    &self.id, content,
                )))
            }
            MessageType::AI => {
                let content: String = serde_json::from_str(&self.content)?;
                let tool_calls: Vec<ToolCall> = if let Some(tc) = &self.tool_calls {
                    serde_json::from_str(tc)?
                } else {
                    Vec::new()
                };
                let msg = if tool_calls.is_empty() {
                    AIMessage::with_id(&self.id, content)
                } else {
                    AIMessage::with_id_and_tool_calls(&self.id, content, tool_calls)
                };
                Ok(BaseMessage::AI(msg))
            }
            MessageType::Tool => {
                let content: String = serde_json::from_str(&self.content)?;
                let tool_call_id = self.tool_call_id.as_ref().ok_or_else(|| {
                    serde_json::Error::custom("Missing tool_call_id for ToolMessage")
                })?;
                Ok(BaseMessage::Tool(ToolMessage::with_id(
                    &self.id,
                    content,
                    tool_call_id,
                )))
            }
        }
    }

    /// Create a database message from an agent-chain BaseMessage.
    ///
    /// The caller must provide conversation_id and sequence_num.
    pub fn from_base_message(
        msg: &BaseMessage,
        conversation_id: String,
        sequence_num: i64,
    ) -> Result<Self, serde_json::Error> {
        let now = Utc::now();
        let id = msg
            .id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        match msg {
            BaseMessage::Human(m) => {
                let content = serde_json::to_string(m.message_content())?;
                Ok(Message {
                    id,
                    conversation_id,
                    message_type: MessageType::Human.to_string(),
                    content,
                    tool_call_id: None,
                    tool_calls: None,
                    additional_kwargs: "{}".to_string(),
                    sequence_num,
                    created_at: now,
                    updated_at: now,
                })
            }
            BaseMessage::System(m) => {
                let content = serde_json::to_string(m.content())?;
                Ok(Message {
                    id,
                    conversation_id,
                    message_type: MessageType::System.to_string(),
                    content,
                    tool_call_id: None,
                    tool_calls: None,
                    additional_kwargs: "{}".to_string(),
                    sequence_num,
                    created_at: now,
                    updated_at: now,
                })
            }
            BaseMessage::AI(m) => {
                let content = serde_json::to_string(m.content())?;
                let tool_calls = if m.tool_calls().is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(m.tool_calls())?)
                };
                let additional_kwargs = serde_json::to_string(m.additional_kwargs())?;
                Ok(Message {
                    id,
                    conversation_id,
                    message_type: MessageType::AI.to_string(),
                    content,
                    tool_call_id: None,
                    tool_calls,
                    additional_kwargs,
                    sequence_num,
                    created_at: now,
                    updated_at: now,
                })
            }
            BaseMessage::Tool(m) => {
                let content = serde_json::to_string(m.content())?;
                Ok(Message {
                    id,
                    conversation_id,
                    message_type: MessageType::Tool.to_string(),
                    content,
                    tool_call_id: Some(m.tool_call_id().to_string()),
                    tool_calls: None,
                    additional_kwargs: "{}".to_string(),
                    sequence_num,
                    created_at: now,
                    updated_at: now,
                })
            }
        }
    }
}

/// Activity table structure (tracking application/process usage).
#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub icon_path: Option<String>,
    pub process_name: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// Activity to conversation mapping.
#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ActivityConversation {
    pub activity_id: String,
    pub conversation_id: String,
    pub created_at: DateTime<Utc>,
}

/// File asset (screenshots, files, etc.).
#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Asset {
    pub id: String,
    pub activity_id: Option<String>,
    pub relative_path: String,
    pub absolute_path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Message to asset mapping.
#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct MessageAsset {
    pub message_id: String,
    pub asset_id: String,
    pub created_at: DateTime<Utc>,
}

/// A conversation with its messages, ready for use with agent-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ConversationWithMessages {
    pub conversation: Conversation,
    pub messages: Vec<BaseMessage>,
}
