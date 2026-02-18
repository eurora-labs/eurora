//! Chat prompt template.
//!
//! This module provides chat prompt templates for chat-based models,
//! mirroring `langchain_core.prompts.chat` in Python.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::messages::{AIMessage, BaseMessage, ChatMessage, HumanMessage, SystemMessage};
use crate::prompt_values::{ChatPromptValue, PromptValue};
use crate::utils::input::get_colored_text;
use crate::utils::interactive_env::is_interactive_env;

use async_trait::async_trait;

use crate::runnables::base::Runnable;
use crate::runnables::config::{RunnableConfig, ensure_config};

use super::base::BasePromptTemplate;
use super::message::{BaseMessagePromptTemplate, get_msg_title_repr};
use super::prompt::PromptTemplate;
use super::string::{PromptTemplateFormat, StringPromptTemplate};

/// Prompt template that assumes variable is already a list of messages.
///
/// A placeholder which can be used to pass in a list of messages.
///
/// Direct port of `langchain_core.prompts.chat.MessagesPlaceholder`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesPlaceholder {
    /// Name of variable to use as messages.
    pub variable_name: String,

    /// If `true`, format_messages can be called with no arguments and will return
    /// an empty list. If `false` then a named argument with name `variable_name`
    /// must be passed in, even if the value is an empty list.
    #[serde(default)]
    pub optional: bool,

    /// Maximum number of messages to include. If `None`, then will include all.
    #[serde(default)]
    pub n_messages: Option<usize>,
}

impl MessagesPlaceholder {
    pub fn new(variable_name: impl Into<String>) -> Self {
        Self {
            variable_name: variable_name.into(),
            optional: false,
            n_messages: None,
        }
    }

    pub fn optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }

    pub fn n_messages(mut self, n: usize) -> Self {
        self.n_messages = Some(n);
        self
    }

    /// Format messages from an explicit messages argument.
    pub fn format_with_messages(
        &self,
        messages: Option<Vec<BaseMessage>>,
    ) -> Result<Vec<BaseMessage>> {
        let value = if self.optional {
            messages.unwrap_or_default()
        } else {
            messages.ok_or_else(|| {
                Error::InvalidConfig(format!(
                    "Variable '{}' is required but was not provided",
                    self.variable_name
                ))
            })?
        };

        let result = if let Some(n) = self.n_messages {
            let len = value.len();
            if len > n {
                value.into_iter().skip(len - n).collect()
            } else {
                value
            }
        } else {
            value
        };

        Ok(result)
    }
}

impl BaseMessagePromptTemplate for MessagesPlaceholder {
    fn input_variables(&self) -> Vec<String> {
        if self.optional {
            Vec::new()
        } else {
            vec![self.variable_name.clone()]
        }
    }

    fn format_messages(&self, _kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        if self.optional {
            Ok(Vec::new())
        } else {
            Err(Error::InvalidConfig(format!(
                "MessagesPlaceholder '{}' requires messages to be passed via format_with_messages",
                self.variable_name
            )))
        }
    }

    fn pretty_repr(&self, html: bool) -> String {
        let var = format!("{{{}}}", self.variable_name);
        let title = get_msg_title_repr("Messages Placeholder", html);
        let var_display = if html {
            get_colored_text(&var, "yellow")
        } else {
            var
        };
        format!("{}\n\n{}", title, var_display)
    }
}

/// Base class for message prompt templates that use a string prompt template.
///
/// Direct port of `langchain_core.prompts.chat.BaseStringMessagePromptTemplate`.
pub trait BaseStringMessagePromptTemplate: BaseMessagePromptTemplate {
    /// Get the underlying string prompt template.
    fn prompt(&self) -> &PromptTemplate;

    /// Get additional kwargs to pass to the message.
    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        static EMPTY: std::sync::LazyLock<HashMap<String, serde_json::Value>> =
            std::sync::LazyLock::new(HashMap::new);
        &EMPTY
    }

    /// Format the prompt template into a message.
    fn format(&self, kwargs: &HashMap<String, String>) -> Result<BaseMessage>;

    /// Async format the prompt template into a message.
    fn aformat(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BaseMessage>> + Send + '_>> {
        let result = self.format(kwargs);
        Box::pin(async move { result })
    }
}

/// Chat message prompt template with a specific role.
///
/// Direct port of `langchain_core.prompts.chat.ChatMessagePromptTemplate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessagePromptTemplate {
    pub prompt: PromptTemplate,
    pub role: String,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
}

impl ChatMessagePromptTemplate {
    pub fn new(prompt: PromptTemplate, role: impl Into<String>) -> Self {
        Self {
            prompt,
            role: role.into(),
            additional_kwargs: HashMap::new(),
        }
    }

    pub fn from_template(
        template: impl Into<String>,
        role: impl Into<String>,
        template_format: PromptTemplateFormat,
    ) -> Result<Self> {
        let prompt = PromptTemplate::from_template_with_format(template, template_format)?;
        Ok(Self::new(prompt, role))
    }
}

impl BaseMessagePromptTemplate for ChatMessagePromptTemplate {
    fn input_variables(&self) -> Vec<String> {
        self.prompt.input_variables.clone()
    }

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        let text = StringPromptTemplate::format(&self.prompt, kwargs)?;
        Ok(vec![BaseMessage::Chat(
            ChatMessage::builder()
                .content(text)
                .role(&self.role)
                .build(),
        )])
    }

    fn pretty_repr(&self, html: bool) -> String {
        let title = format!("{} Message", self.role);
        let title = get_msg_title_repr(&title, html);
        format!("{}\n\n{}", title, self.prompt.pretty_repr(html))
    }
}

impl BaseStringMessagePromptTemplate for ChatMessagePromptTemplate {
    fn prompt(&self) -> &PromptTemplate {
        &self.prompt
    }

    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<BaseMessage> {
        let text = StringPromptTemplate::format(&self.prompt, kwargs)?;
        Ok(BaseMessage::Chat(
            ChatMessage::builder()
                .content(text)
                .role(&self.role)
                .build(),
        ))
    }
}

/// Human message prompt template.
///
/// Direct port of `langchain_core.prompts.chat.HumanMessagePromptTemplate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanMessagePromptTemplate {
    pub prompt: PromptTemplate,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
}

impl HumanMessagePromptTemplate {
    pub fn new(prompt: PromptTemplate) -> Self {
        Self {
            prompt,
            additional_kwargs: HashMap::new(),
        }
    }

    pub fn from_template(template: impl Into<String>) -> Result<Self> {
        Self::from_template_with_format(template, PromptTemplateFormat::FString)
    }

    pub fn from_template_with_format(
        template: impl Into<String>,
        template_format: PromptTemplateFormat,
    ) -> Result<Self> {
        let prompt = PromptTemplate::from_template_with_format(template, template_format)?;
        Ok(Self::new(prompt))
    }

    pub fn from_template_file(template_file: impl AsRef<Path>) -> Result<Self> {
        let prompt = PromptTemplate::from_file(template_file)?;
        Ok(Self::new(prompt))
    }
}

impl BaseMessagePromptTemplate for HumanMessagePromptTemplate {
    fn input_variables(&self) -> Vec<String> {
        self.prompt.input_variables.clone()
    }

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        let text = StringPromptTemplate::format(&self.prompt, kwargs)?;
        Ok(vec![BaseMessage::Human(
            HumanMessage::builder().content(text).build(),
        )])
    }

    fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("Human Message", html);
        format!("{}\n\n{}", title, self.prompt.pretty_repr(html))
    }
}

impl BaseStringMessagePromptTemplate for HumanMessagePromptTemplate {
    fn prompt(&self) -> &PromptTemplate {
        &self.prompt
    }

    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<BaseMessage> {
        let text = StringPromptTemplate::format(&self.prompt, kwargs)?;
        Ok(BaseMessage::Human(
            HumanMessage::builder().content(text).build(),
        ))
    }
}

/// AI message prompt template.
///
/// Direct port of `langchain_core.prompts.chat.AIMessagePromptTemplate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessagePromptTemplate {
    pub prompt: PromptTemplate,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
}

impl AIMessagePromptTemplate {
    pub fn new(prompt: PromptTemplate) -> Self {
        Self {
            prompt,
            additional_kwargs: HashMap::new(),
        }
    }

    pub fn from_template(template: impl Into<String>) -> Result<Self> {
        Self::from_template_with_format(template, PromptTemplateFormat::FString)
    }

    pub fn from_template_with_format(
        template: impl Into<String>,
        template_format: PromptTemplateFormat,
    ) -> Result<Self> {
        let prompt = PromptTemplate::from_template_with_format(template, template_format)?;
        Ok(Self::new(prompt))
    }

    pub fn from_template_file(template_file: impl AsRef<Path>) -> Result<Self> {
        let prompt = PromptTemplate::from_file(template_file)?;
        Ok(Self::new(prompt))
    }
}

impl BaseMessagePromptTemplate for AIMessagePromptTemplate {
    fn input_variables(&self) -> Vec<String> {
        self.prompt.input_variables.clone()
    }

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        let text = StringPromptTemplate::format(&self.prompt, kwargs)?;
        Ok(vec![BaseMessage::AI(
            AIMessage::builder().content(text).build(),
        )])
    }

    fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("AI Message", html);
        format!("{}\n\n{}", title, self.prompt.pretty_repr(html))
    }
}

impl BaseStringMessagePromptTemplate for AIMessagePromptTemplate {
    fn prompt(&self) -> &PromptTemplate {
        &self.prompt
    }

    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<BaseMessage> {
        let text = StringPromptTemplate::format(&self.prompt, kwargs)?;
        Ok(BaseMessage::AI(AIMessage::builder().content(text).build()))
    }
}

/// System message prompt template.
///
/// Direct port of `langchain_core.prompts.chat.SystemMessagePromptTemplate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessagePromptTemplate {
    pub prompt: PromptTemplate,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
}

impl SystemMessagePromptTemplate {
    pub fn new(prompt: PromptTemplate) -> Self {
        Self {
            prompt,
            additional_kwargs: HashMap::new(),
        }
    }

    pub fn from_template(template: impl Into<String>) -> Result<Self> {
        Self::from_template_with_format(template, PromptTemplateFormat::FString)
    }

    pub fn from_template_with_format(
        template: impl Into<String>,
        template_format: PromptTemplateFormat,
    ) -> Result<Self> {
        let prompt = PromptTemplate::from_template_with_format(template, template_format)?;
        Ok(Self::new(prompt))
    }

    pub fn from_template_file(template_file: impl AsRef<Path>) -> Result<Self> {
        let prompt = PromptTemplate::from_file(template_file)?;
        Ok(Self::new(prompt))
    }
}

impl BaseMessagePromptTemplate for SystemMessagePromptTemplate {
    fn input_variables(&self) -> Vec<String> {
        self.prompt.input_variables.clone()
    }

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        let text = StringPromptTemplate::format(&self.prompt, kwargs)?;
        Ok(vec![BaseMessage::System(
            SystemMessage::builder().content(text).build(),
        )])
    }

    fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("System Message", html);
        format!("{}\n\n{}", title, self.prompt.pretty_repr(html))
    }
}

impl BaseStringMessagePromptTemplate for SystemMessagePromptTemplate {
    fn prompt(&self) -> &PromptTemplate {
        &self.prompt
    }

    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<BaseMessage> {
        let text = StringPromptTemplate::format(&self.prompt, kwargs)?;
        Ok(BaseMessage::System(
            SystemMessage::builder().content(text).build(),
        ))
    }
}

/// A message-like type that can be part of a chat prompt.
///
/// Corresponds to `MessageLike` in Python:
/// `BaseMessagePromptTemplate | BaseMessage | BaseChatPromptTemplate`
#[derive(Clone)]
pub enum MessageLike {
    /// A base message.
    Message(Box<BaseMessage>),
    /// A message prompt template.
    Template(Box<dyn MessageLikeClone + Send + Sync>),
    /// A messages placeholder.
    Placeholder(MessagesPlaceholder),
}

impl std::fmt::Debug for MessageLike {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageLike::Message(m) => f.debug_tuple("Message").field(m).finish(),
            MessageLike::Template(_) => f.debug_tuple("Template").field(&"<template>").finish(),
            MessageLike::Placeholder(p) => f.debug_tuple("Placeholder").field(p).finish(),
        }
    }
}

/// Helper trait for cloning message-like templates.
pub trait MessageLikeClone: BaseMessagePromptTemplate {
    fn clone_box(&self) -> Box<dyn MessageLikeClone + Send + Sync>;
}

impl<T> MessageLikeClone for T
where
    T: BaseMessagePromptTemplate + Clone + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn MessageLikeClone + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn MessageLikeClone + Send + Sync> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Representation of a message-like that can be converted to a message template.
///
/// Corresponds to `MessageLikeRepresentation` in Python:
/// `MessageLike | tuple[str|type, str|list] | str | dict[str, Any]`
#[derive(Clone)]
pub enum MessageLikeRepresentation {
    /// A (role, content) tuple — e.g., `("human", "{user_input}")`.
    Tuple(String, String),
    /// A string (shorthand for human message) — e.g., `"{user_input}"`.
    String(String),
    /// A base message passed directly.
    Message(Box<BaseMessage>),
    /// A placeholder: `("placeholder", "{variable_name}")`.
    Placeholder {
        variable_name: String,
        optional: bool,
    },
    /// A message prompt template.
    Template(Box<dyn MessageLikeClone + Send + Sync>),
}

impl MessageLikeRepresentation {
    pub fn tuple(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Tuple(role.into(), content.into())
    }

    pub fn string(content: impl Into<String>) -> Self {
        Self::String(content.into())
    }

    pub fn placeholder(variable_name: impl Into<String>, optional: bool) -> Self {
        Self::Placeholder {
            variable_name: variable_name.into(),
            optional,
        }
    }
}

/// Allow `(&str, &str)` tuples to be converted to `MessageLikeRepresentation`.
impl From<(&str, &str)> for MessageLikeRepresentation {
    fn from((role, content): (&str, &str)) -> Self {
        Self::Tuple(role.to_string(), content.to_string())
    }
}

/// Allow `BaseMessage` to be converted to `MessageLikeRepresentation`.
impl From<BaseMessage> for MessageLikeRepresentation {
    fn from(msg: BaseMessage) -> Self {
        Self::Message(Box::new(msg))
    }
}

/// Allow `&str` to be converted to `MessageLikeRepresentation` (human message).
impl From<&str> for MessageLikeRepresentation {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl std::fmt::Debug for MessageLikeRepresentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tuple(role, content) => {
                f.debug_tuple("Tuple").field(role).field(content).finish()
            }
            Self::String(s) => f.debug_tuple("String").field(s).finish(),
            Self::Message(m) => f.debug_tuple("Message").field(m).finish(),
            Self::Placeholder {
                variable_name,
                optional,
            } => f
                .debug_struct("Placeholder")
                .field("variable_name", variable_name)
                .field("optional", optional)
                .finish(),
            Self::Template(_) => f.debug_tuple("Template").field(&"<template>").finish(),
        }
    }
}

/// Base trait for chat prompt templates.
///
/// Direct port of `langchain_core.prompts.chat.BaseChatPromptTemplate`.
pub trait BaseChatPromptTemplate: BasePromptTemplate {
    /// Format kwargs into a list of messages.
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>>;

    /// Async format kwargs into a list of messages.
    fn aformat_messages(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<BaseMessage>>> + Send + '_>>
    {
        let result = self.format_messages(kwargs);
        Box::pin(async move { result })
    }

    /// Format prompt. Returns a ChatPromptValue.
    fn format_prompt_chat(&self, kwargs: &HashMap<String, String>) -> Result<ChatPromptValue> {
        let messages = self.format_messages(kwargs)?;
        Ok(ChatPromptValue::new(messages))
    }

    /// Async format prompt. Returns a ChatPromptValue.
    fn aformat_prompt_chat(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ChatPromptValue>> + Send + '_>>
    {
        let result = self.format_prompt_chat(kwargs);
        Box::pin(async move { result })
    }

    /// Get a pretty representation of the template.
    fn pretty_repr(&self, html: bool) -> String;

    /// Print a human-readable representation.
    fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }
}

/// A message in a chat prompt template.
///
/// Internal enum dispatching to the concrete message template types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatPromptMessage {
    /// A static message.
    Message(BaseMessage),
    /// A human message template.
    Human(HumanMessagePromptTemplate),
    /// An AI message template.
    AI(AIMessagePromptTemplate),
    /// A system message template.
    System(SystemMessagePromptTemplate),
    /// A chat message template with role.
    Chat(ChatMessagePromptTemplate),
    /// A messages placeholder.
    Placeholder(MessagesPlaceholder),
}

impl ChatPromptMessage {
    fn input_variables(&self) -> Vec<String> {
        match self {
            ChatPromptMessage::Message(_) => Vec::new(),
            ChatPromptMessage::Human(t) => t.input_variables(),
            ChatPromptMessage::AI(t) => t.input_variables(),
            ChatPromptMessage::System(t) => t.input_variables(),
            ChatPromptMessage::Chat(t) => t.input_variables(),
            ChatPromptMessage::Placeholder(p) => p.input_variables(),
        }
    }

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        match self {
            ChatPromptMessage::Message(m) => Ok(vec![m.clone()]),
            ChatPromptMessage::Human(t) => t.format_messages(kwargs),
            ChatPromptMessage::AI(t) => t.format_messages(kwargs),
            ChatPromptMessage::System(t) => t.format_messages(kwargs),
            ChatPromptMessage::Chat(t) => t.format_messages(kwargs),
            ChatPromptMessage::Placeholder(p) => p.format_messages(kwargs),
        }
    }

    fn pretty_repr(&self, html: bool) -> String {
        match self {
            ChatPromptMessage::Message(m) => m.pretty_repr(html),
            ChatPromptMessage::Human(t) => t.pretty_repr(html),
            ChatPromptMessage::AI(t) => t.pretty_repr(html),
            ChatPromptMessage::System(t) => t.pretty_repr(html),
            ChatPromptMessage::Chat(t) => t.pretty_repr(html),
            ChatPromptMessage::Placeholder(p) => p.pretty_repr(html),
        }
    }
}

/// Chat prompt template for chat models.
///
/// Use to create flexible templated prompts for chat models.
///
/// Direct port of `langchain_core.prompts.chat.ChatPromptTemplate`.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::prompts::ChatPromptTemplate;
///
/// let template = ChatPromptTemplate::from_messages(vec![
///     ("system", "You are a helpful AI bot. Your name is {name}.").into(),
///     ("human", "Hello, how are you doing?").into(),
///     ("ai", "I'm doing well, thanks!").into(),
///     ("human", "{user_input}").into(),
/// ]).unwrap();
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatPromptTemplate {
    /// List of messages or message templates.
    pub messages: Vec<ChatPromptMessage>,
    input_variables: Vec<String>,
    optional_variables: Vec<String>,
    partial_variables: HashMap<String, String>,
    validate_template: bool,
    template_format: PromptTemplateFormat,
}

impl ChatPromptTemplate {
    /// Create a new empty chat prompt template.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a chat prompt template from a variety of message formats.
    ///
    /// Accepts `MessageLikeRepresentation` items which can be:
    /// 1. `BaseMessagePromptTemplate` (via `MessageLikeRepresentation::Template`)
    /// 2. `BaseMessage` (via `MessageLikeRepresentation::Message` or `.into()`)
    /// 3. 2-tuple of `(role, template)` — e.g., `("human", "{user_input}")`
    /// 4. A string shorthand for `("human", template)`
    ///
    /// Direct port of Python `ChatPromptTemplate.from_messages()`.
    pub fn from_messages(messages: Vec<MessageLikeRepresentation>) -> Result<Self> {
        Self::from_messages_with_format(messages, PromptTemplateFormat::FString)
    }

    /// Create a chat prompt template with a specific template format.
    pub fn from_messages_with_format(
        messages: Vec<MessageLikeRepresentation>,
        template_format: PromptTemplateFormat,
    ) -> Result<Self> {
        let mut chat_messages = Vec::with_capacity(messages.len());

        for msg in messages {
            let chat_msg = convert_to_message_template(msg, template_format)?;
            chat_messages.push(chat_msg);
        }

        let mut input_vars = std::collections::HashSet::new();
        let mut optional_vars = std::collections::HashSet::new();
        let mut partial_vars = HashMap::new();

        for msg in &chat_messages {
            match msg {
                ChatPromptMessage::Placeholder(p) if p.optional => {
                    partial_vars.insert(p.variable_name.clone(), String::new());
                    optional_vars.insert(p.variable_name.clone());
                }
                _ => {
                    for var in msg.input_variables() {
                        input_vars.insert(var);
                    }
                }
            }
        }

        let mut input_variables: Vec<_> = input_vars.into_iter().collect();
        input_variables.sort();

        let mut optional_variables: Vec<_> = optional_vars.into_iter().collect();
        optional_variables.sort();

        Ok(Self {
            messages: chat_messages,
            input_variables,
            optional_variables,
            partial_variables: partial_vars,
            validate_template: false,
            template_format,
        })
    }

    /// Create a chat prompt template from a single template string.
    ///
    /// Creates a chat template consisting of a single message assumed to be from
    /// the human.
    ///
    /// Direct port of Python `ChatPromptTemplate.from_template()`.
    pub fn from_template(template: &str) -> Result<Self> {
        Self::from_messages(vec![MessageLikeRepresentation::Tuple(
            "human".to_string(),
            template.to_string(),
        )])
    }

    /// Append a message to the end of the chat template.
    ///
    /// Direct port of Python `ChatPromptTemplate.append()`.
    pub fn append(&mut self, message: MessageLikeRepresentation) -> Result<()> {
        let chat_msg = convert_to_message_template(message, self.template_format)?;
        match &chat_msg {
            ChatPromptMessage::Placeholder(p) if p.optional => {
                if !self.optional_variables.contains(&p.variable_name) {
                    self.optional_variables.push(p.variable_name.clone());
                }
            }
            _ => {
                for var in chat_msg.input_variables() {
                    if !self.input_variables.contains(&var) {
                        self.input_variables.push(var);
                    }
                }
            }
        }
        self.messages.push(chat_msg);
        Ok(())
    }

    /// Extend the chat template with a sequence of messages.
    ///
    /// Direct port of Python `ChatPromptTemplate.extend()`.
    pub fn extend(&mut self, messages: Vec<MessageLikeRepresentation>) -> Result<()> {
        for msg in messages {
            self.append(msg)?;
        }
        Ok(())
    }

    /// Get a new ChatPromptTemplate with some input variables already filled in.
    ///
    /// Direct port of Python `ChatPromptTemplate.partial()`.
    pub fn partial(&self, kwargs: HashMap<String, String>) -> Self {
        let new_vars: Vec<_> = self
            .input_variables
            .iter()
            .filter(|v| !kwargs.contains_key(*v))
            .cloned()
            .collect();

        let mut new_partials = self.partial_variables.clone();
        new_partials.extend(kwargs);

        Self {
            messages: self.messages.clone(),
            input_variables: new_vars,
            optional_variables: self.optional_variables.clone(),
            partial_variables: new_partials,
            validate_template: self.validate_template,
            template_format: self.template_format,
        }
    }

    /// Get the number of messages in the template.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the template is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get a message by index.
    pub fn get(&self, index: usize) -> Option<&ChatPromptMessage> {
        self.messages.get(index)
    }

    fn merge_partial_and_user_variables(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut merged = self.partial_variables.clone();
        merged.extend(kwargs.clone());
        merged
    }
}

impl BaseChatPromptTemplate for ChatPromptTemplate {
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        let merged = self.merge_partial_and_user_variables(kwargs);
        let mut result = Vec::new();

        for message in &self.messages {
            let formatted = message.format_messages(&merged)?;
            result.extend(formatted);
        }

        Ok(result)
    }

    fn pretty_repr(&self, html: bool) -> String {
        self.messages
            .iter()
            .map(|m| m.pretty_repr(html))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[async_trait]
impl Runnable for ChatPromptTemplate {
    type Input = HashMap<String, String>;
    type Output = ChatPromptValue;

    fn name(&self) -> Option<String> {
        Some("ChatPromptTemplate".to_string())
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let _config = ensure_config(config);
        self.validate_input(&input)?;
        let messages = BaseChatPromptTemplate::format_messages(self, &input)?;
        Ok(ChatPromptValue::new(messages))
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }
}

/// Create a message prompt template from a message type string and template.
///
/// Direct port of Python `_create_template_from_message_type`.
fn create_template_from_message_type(
    message_type: &str,
    template: &str,
    template_format: PromptTemplateFormat,
) -> Result<ChatPromptMessage> {
    match message_type {
        "human" | "user" => {
            let t =
                HumanMessagePromptTemplate::from_template_with_format(template, template_format)?;
            Ok(ChatPromptMessage::Human(t))
        }
        "ai" | "assistant" => {
            let t = AIMessagePromptTemplate::from_template_with_format(template, template_format)?;
            Ok(ChatPromptMessage::AI(t))
        }
        "system" => {
            let t =
                SystemMessagePromptTemplate::from_template_with_format(template, template_format)?;
            Ok(ChatPromptMessage::System(t))
        }
        "placeholder" => {
            if !template.starts_with('{') || !template.ends_with('}') {
                return Err(Error::InvalidConfig(format!(
                    "Invalid placeholder template: {}. Expected a variable name surrounded by curly braces.",
                    template
                )));
            }
            let var_name = &template[1..template.len() - 1];
            let placeholder = MessagesPlaceholder::new(var_name).optional(true);
            Ok(ChatPromptMessage::Placeholder(placeholder))
        }
        _ => Err(Error::InvalidConfig(format!(
            "Unexpected message type: {}. Use one of 'human', 'user', 'ai', 'assistant', 'system', or 'placeholder'.",
            message_type
        ))),
    }
}

/// Instantiate a ChatPromptMessage from a MessageLikeRepresentation.
///
/// Direct port of Python `_convert_to_message_template`.
fn convert_to_message_template(
    message: MessageLikeRepresentation,
    template_format: PromptTemplateFormat,
) -> Result<ChatPromptMessage> {
    match message {
        MessageLikeRepresentation::Tuple(role, content) => {
            create_template_from_message_type(&role, &content, template_format)
        }
        MessageLikeRepresentation::String(content) => {
            create_template_from_message_type("human", &content, template_format)
        }
        MessageLikeRepresentation::Message(msg) => Ok(ChatPromptMessage::Message(*msg)),
        MessageLikeRepresentation::Placeholder {
            variable_name,
            optional,
        } => {
            let placeholder = MessagesPlaceholder::new(variable_name).optional(optional);
            Ok(ChatPromptMessage::Placeholder(placeholder))
        }
        MessageLikeRepresentation::Template(_t) => {
            Err(Error::InvalidConfig(
                "Template variant should be passed as a concrete ChatPromptMessage.                  Use Tuple, Message, or String variants instead.".into(),
            ))
        }
    }
}

/// Implements `BasePromptTemplate` for `ChatPromptTemplate`, matching
/// Python's inheritance chain where `BaseChatPromptTemplate` extends
/// `BasePromptTemplate`.
impl BasePromptTemplate for ChatPromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn optional_variables(&self) -> &[String] {
        &self.optional_variables
    }

    fn partial_variables(&self) -> &HashMap<String, String> {
        &self.partial_variables
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String> {
        let messages = self.format_messages(kwargs)?;
        let prompt_value = ChatPromptValue::new(messages);
        Ok(prompt_value.to_string())
    }

    fn format_prompt(&self, kwargs: &HashMap<String, String>) -> Result<Box<dyn PromptValue>> {
        let messages = self.format_messages(kwargs)?;
        Ok(Box::new(ChatPromptValue::new(messages)))
    }

    fn partial(&self, kwargs: HashMap<String, String>) -> Result<Box<dyn BasePromptTemplate>> {
        Ok(Box::new(ChatPromptTemplate::partial(self, kwargs)))
    }

    fn prompt_type(&self) -> &str {
        "chat"
    }

    fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.prompt_type(),
            "input_variables": self.input_variables,
        })
    }
}

impl std::ops::Add for ChatPromptTemplate {
    type Output = ChatPromptTemplate;

    /// Combine two prompt templates.
    ///
    /// Direct port of Python `ChatPromptTemplate.__add__`.
    fn add(self, other: Self) -> Self::Output {
        let mut messages = self.messages;
        messages.extend(other.messages);

        let mut input_vars: std::collections::HashSet<_> =
            self.input_variables.into_iter().collect();
        input_vars.extend(other.input_variables);

        let mut partial_vars = self.partial_variables;
        partial_vars.extend(other.partial_variables);

        let mut optional_vars: std::collections::HashSet<_> =
            self.optional_variables.into_iter().collect();
        optional_vars.extend(other.optional_variables);

        ChatPromptTemplate {
            messages,
            input_variables: {
                let mut v: Vec<_> = input_vars.into_iter().collect();
                v.sort();
                v
            },
            optional_variables: {
                let mut v: Vec<_> = optional_vars.into_iter().collect();
                v.sort();
                v
            },
            partial_variables: partial_vars,
            validate_template: false,
            template_format: self.template_format,
        }
    }
}

use crate::load::Serializable;
use serde_json::Value;

impl Serializable for MessagesPlaceholder {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "chat".to_string(),
        ]
    }
}

impl Serializable for HumanMessagePromptTemplate {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "chat".to_string(),
        ]
    }
}

impl Serializable for AIMessagePromptTemplate {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "chat".to_string(),
        ]
    }
}

impl Serializable for SystemMessagePromptTemplate {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "chat".to_string(),
        ]
    }
}

impl Serializable for ChatMessagePromptTemplate {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "chat".to_string(),
        ]
    }
}

impl Serializable for ChatPromptTemplate {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "chat".to_string(),
        ]
    }

    fn lc_attributes(&self) -> std::collections::HashMap<String, Value> {
        let mut attrs = std::collections::HashMap::new();
        attrs.insert(
            "input_variables".to_string(),
            serde_json::to_value(&self.input_variables).unwrap_or_default(),
        );
        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages_placeholder() {
        let placeholder = MessagesPlaceholder::new("history");
        assert_eq!(placeholder.input_variables(), vec!["history"]);

        let optional_placeholder = MessagesPlaceholder::new("history").optional(true);
        assert!(optional_placeholder.input_variables().is_empty());
    }

    #[test]
    fn test_human_message_template() {
        let template = HumanMessagePromptTemplate::from_template("Hello, {name}!").unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let messages = template.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content(), "Hello, World!");
    }

    #[test]
    fn test_system_message_template() {
        let template = SystemMessagePromptTemplate::from_template("You are {role}").unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("role".to_string(), "an assistant".to_string());

        let messages = template.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content(), "You are an assistant");
    }

    #[test]
    fn test_chat_prompt_template() {
        let template = ChatPromptTemplate::from_messages(vec![
            ("system", "You are a helpful assistant.").into(),
            ("human", "{question}").into(),
        ])
        .unwrap();

        assert_eq!(template.input_variables(), &["question"]);

        let mut kwargs = HashMap::new();
        kwargs.insert("question".to_string(), "Hello!".to_string());

        let messages = template.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content(), "You are a helpful assistant.");
        assert_eq!(messages[1].content(), "Hello!");
    }

    #[test]
    fn test_chat_prompt_template_from_template() {
        let template = ChatPromptTemplate::from_template("Hello, {name}!").unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let messages = template.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content(), "Hello, World!");
    }

    #[test]
    fn test_chat_prompt_add() {
        let template1 = ChatPromptTemplate::from_messages(vec![
            ("system", "You are a helpful assistant.").into(),
        ])
        .unwrap();

        let template2 =
            ChatPromptTemplate::from_messages(vec![("human", "{question}").into()]).unwrap();

        let combined = template1 + template2;

        let mut kwargs = HashMap::new();
        kwargs.insert("question".to_string(), "Hello!".to_string());

        let messages = combined.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_partial() {
        let template = ChatPromptTemplate::from_messages(vec![
            ("system", "You are {role}.").into(),
            ("human", "{question}").into(),
        ])
        .unwrap();

        let mut partial_vars = HashMap::new();
        partial_vars.insert("role".to_string(), "an assistant".to_string());

        let partial = template.partial(partial_vars);
        assert_eq!(partial.input_variables(), &["question"]);

        let mut kwargs = HashMap::new();
        kwargs.insert("question".to_string(), "Hello!".to_string());

        let messages = partial.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content(), "You are an assistant.");
    }

    #[test]
    fn test_from_messages_with_base_message() {
        let template = ChatPromptTemplate::from_messages(vec![
            BaseMessage::System(SystemMessage::builder().content("hello").build()).into(),
            ("human", "Hi {name}").into(),
        ])
        .unwrap();

        assert_eq!(template.input_variables(), &["name"]);
        assert_eq!(template.len(), 2);

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "Bob".to_string());
        let messages = template.format_messages(&kwargs).unwrap();
        assert_eq!(messages[0].content(), "hello");
        assert_eq!(messages[1].content(), "Hi Bob");
    }

    #[test]
    fn test_from_messages_with_string() {
        let template = ChatPromptTemplate::from_messages(vec![MessageLikeRepresentation::String(
            "Hello {name}".to_string(),
        )])
        .unwrap();

        assert_eq!(template.input_variables(), &["name"]);

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());
        let messages = template.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content(), "Hello World");
    }

    #[test]
    fn test_from_messages_with_placeholder() {
        let template = ChatPromptTemplate::from_messages(vec![
            ("system", "You are a helpful assistant.").into(),
            ("placeholder", "{history}").into(),
            ("human", "{question}").into(),
        ])
        .unwrap();

        assert_eq!(template.input_variables(), &["question"]);
        assert!(
            template
                .optional_variables()
                .contains(&"history".to_string())
        );
    }

    #[test]
    fn test_format_prompt() {
        let template = ChatPromptTemplate::from_messages(vec![
            ("system", "You are helpful.").into(),
            ("human", "{question}").into(),
        ])
        .unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("question".to_string(), "Hello!".to_string());

        let messages = template.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_extend() {
        let mut template =
            ChatPromptTemplate::from_messages(vec![("system", "You are helpful.").into()]).unwrap();

        template
            .extend(vec![
                ("human", "{question}").into(),
                ("ai", "I can help with that.").into(),
            ])
            .unwrap();

        assert_eq!(template.len(), 3);
        assert_eq!(template.input_variables(), &["question"]);
    }

    #[test]
    fn test_append() {
        let mut template =
            ChatPromptTemplate::from_messages(vec![("system", "You are helpful.").into()]).unwrap();

        template.append(("human", "{question}").into()).unwrap();

        assert_eq!(template.len(), 2);
        assert_eq!(template.input_variables(), &["question"]);
    }
}
