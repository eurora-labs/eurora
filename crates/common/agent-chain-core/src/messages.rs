//! Message types for LLM interactions.
//!
//! This module provides message types for different roles (human, AI, system, tool)
//! as well as types for tool calls. The structure mirrors the Python langchain_core.messages
//! module.
//!
//! # Multimodal Support
//!
//! The [`HumanMessage`] type supports multimodal content including text and images.
//! Images can be provided as URLs or base64-encoded data.
//!
//! ```ignore
//! use agent_chain_core::messages::{HumanMessage, ContentPart, ImageSource};
//!
//! // Simple text message
//! let msg = HumanMessage::new("Hello!");
//!
//! // Message with image from URL
//! let msg = HumanMessage::with_content(vec![
//!     ContentPart::Text { text: "What's in this image?".into() },
//!     ContentPart::Image {
//!         source: ImageSource::Url {
//!             url: "https://example.com/image.jpg".into(),
//!         },
//!         detail: None,
//!     },
//! ]);
//!
//! // Message with base64-encoded image
//! let msg = HumanMessage::with_content(vec![
//!     ContentPart::Text { text: "Describe this image".into() },
//!     ContentPart::Image {
//!         source: ImageSource::Base64 {
//!             media_type: "image/jpeg".into(),
//!             data: base64_image_data,
//!         },
//!         detail: Some(ImageDetail::High),
//!     },
//! ]);
//! ```

// Submodules - organized like langchain_core.messages
mod ai;
mod base;
mod content;
mod human;
mod modifier;
mod system;
mod tool;
mod utils;

// Re-export from ai
pub use ai::{AIMessage, InputTokenDetails, OutputTokenDetails, UsageMetadata};

// Re-export from base
pub use base::{BaseMessage, HasId};

// Re-export from content
pub use content::{ContentPart, ImageDetail, ImageSource, MessageContent};

// Re-export from human
pub use human::HumanMessage;

// Re-export from modifier
pub use modifier::RemoveMessage;

// Re-export from system
pub use system::SystemMessage;

// Re-export from tool
pub use tool::{InvalidToolCall, ToolCall, ToolCallChunk, ToolMessage, ToolStatus};

// Re-export from utils
pub use utils::{get_buffer_string, message_to_dict, messages_to_dict, AnyMessage};