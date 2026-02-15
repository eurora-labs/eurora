//! Prompt templates for LLM interactions.
//!
//! This module provides prompt templates for different use cases, mirroring
//! the structure of `langchain_core.prompts` in Python.
//!
//! # Overview
//!
//! Prompt templates are used to construct prompts for language models. They allow
//! you to define templates with placeholders that can be filled in with values
//! at runtime.
//!
//! # Template Formats
//!
//! Three template formats are supported:
//!
//! - **f-string** (default): Uses Python-style `{variable}` placeholders
//! - **mustache**: Uses `{{variable}}` placeholders with sections and loops
//! - **jinja2**: Full Jinja2 templating (requires `jinja2` feature)
//!
//! # Example
//!
//! ```ignore
//! use agent_chain_core::prompts::{PromptTemplate, ChatPromptTemplate};
//!
//! // Simple string prompt
//! let prompt = PromptTemplate::from_template("Hello, {name}!");
//! let result = prompt.format(&[("name", "World")].into_iter().collect()).unwrap();
//! assert_eq!(result, "Hello, World!");
//!
//! // Chat prompt with multiple messages
//! let chat_prompt = ChatPromptTemplate::from_messages(vec![
//!     ("system", "You are a helpful assistant.").into(),
//!     ("human", "{question}").into(),
//! ]);
//! ```

mod base;
mod chat;
mod dict;
mod few_shot;
mod few_shot_with_templates;
mod image;
mod loading;
mod message;
mod prompt;
mod string;

// Re-export from base
pub use base::{BasePromptTemplate, FormatOutputType, aformat_document, format_document};

// Re-export from string
pub use string::{
    PromptTemplateFormat, StringPromptTemplate, check_valid_template, get_template_variables,
    jinja2_formatter, mustache_formatter, validate_jinja2,
};

// Re-export from prompt
pub use prompt::PromptTemplate;

// Re-export from message
pub use message::BaseMessagePromptTemplate;

// Re-export from chat
pub use chat::{
    AIMessagePromptTemplate, BaseChatPromptTemplate, BaseStringMessagePromptTemplate,
    ChatMessagePromptTemplate, ChatPromptTemplate, HumanMessagePromptTemplate, MessageLike,
    MessageLikeRepresentation, MessagesPlaceholder, SystemMessagePromptTemplate,
};

// Re-export from dict
pub use dict::DictPromptTemplate;

// Re-export from image
pub use image::ImagePromptTemplate;

// Re-export from few_shot
pub use few_shot::{FewShotChatMessagePromptTemplate, FewShotPromptTemplate};

// Re-export from few_shot_with_templates
pub use few_shot_with_templates::FewShotPromptWithTemplates;

// Re-export from loading
pub use loading::load_prompt;
