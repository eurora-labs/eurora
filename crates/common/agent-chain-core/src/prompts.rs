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
mod structured;

pub use base::{BasePromptTemplate, FormatOutputType, aformat_document, format_document};

pub use string::{
    PromptTemplateFormat, StringPromptTemplate, check_valid_template, get_template_variables,
    jinja2_formatter, mustache_formatter, validate_jinja2,
};

pub use prompt::PromptTemplate;

pub use message::BaseMessagePromptTemplate;

pub use chat::{
    AIMessagePromptTemplate, BaseChatPromptTemplate, BaseStringMessagePromptTemplate,
    ChatMessagePromptTemplate, ChatPromptTemplate, HumanMessagePromptTemplate, MessageLike,
    MessageLikeRepresentation, MessagesPlaceholder, SystemMessagePromptTemplate,
};

pub use dict::DictPromptTemplate;

pub use image::ImagePromptTemplate;

pub use few_shot::{FewShotChatMessagePromptTemplate, FewShotPromptTemplate};

pub use few_shot_with_templates::FewShotPromptWithTemplates;

pub use structured::StructuredPrompt;

pub use loading::load_prompt;
