//! Compatibility module for OpenAI chat models.
//!
//! Provides backward-compatible re-exports for transitioning between
//! different versions of the OpenAI API.
//!
//! Matches Python `langchain_openai.chat_models._compat`.

pub use super::base::{BuiltinTool, ChatOpenAI};
