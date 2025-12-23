//! Euro Personal Database
//!
//! This crate provides a SQLite database for storing personal data including
//! conversations, messages, activities, and assets. It is designed to work
//! seamlessly with the agent-chain library for storing LLM conversations.
//!
//! # Example
//!
//! ```ignore
//! use euro_personal_db::PersonalDatabaseManager;
//! use agent_chain_core::{HumanMessage, AIMessage, BaseMessage};
//!
//! // Create a database manager
//! let db = PersonalDatabaseManager::new("./data/personal.db").await?;
//!
//! // Create a conversation
//! let conversation = db.insert_empty_conversation().await?;
//!
//! // Insert messages directly from agent-chain types
//! let messages: Vec<BaseMessage> = vec![
//!     HumanMessage::new("Hello!").into(),
//!     AIMessage::new("Hi! How can I help you?").into(),
//! ];
//!
//! db.insert_base_messages(&conversation.id, &messages).await?;
//!
//! // Retrieve messages as agent-chain types
//! let (conv, msgs) = db.get_conversation_with_messages(&conversation.id).await?;
//! ```

mod db;
mod dto;
mod types;

pub use db::{PERSONAL_DB_KEY_HANDLE, PersonalDatabaseManager};
pub use dto::*;
pub use types::*;
