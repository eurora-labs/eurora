//! **Chat message history** stores a history of the message interactions in a chat.
//!
//! This module provides abstractions for storing chat message history.
//! Mirrors `langchain_core.chat_history`.

use async_trait::async_trait;
use std::fmt::Display;

use crate::messages::{AIMessage, BaseMessage, HumanMessage, get_buffer_string};

/// Abstract base trait for storing chat message history.
///
/// Implementations guidelines:
///
/// Implementations are expected to override all or some of the following methods:
///
/// * `add_messages`: sync variant for bulk addition of messages
/// * `add_messages_async`: async variant for bulk addition of messages
/// * `messages`: sync variant for getting messages
/// * `get_messages_async`: async variant for getting messages
/// * `clear`: sync variant for clearing messages
/// * `clear_async`: async variant for clearing messages
///
/// `add_messages` contains a default implementation that calls `add_message`
/// for each message in the sequence. This is provided for backwards compatibility
/// with existing implementations which only had `add_message`.
///
/// Async variants all have default implementations that call the sync variants.
/// Implementers can choose to override the async implementations to provide
/// truly async implementations.
///
/// Usage guidelines:
///
/// When used for updating history, users should favor usage of `add_messages`
/// over `add_message` or other variants like `add_user_message` and `add_ai_message`
/// to avoid unnecessary round-trips to the underlying persistence layer.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::chat_history::{BaseChatMessageHistory, InMemoryChatMessageHistory};
/// use agent_chain_core::messages::{BaseMessage, HumanMessage, AIMessage};
///
/// let mut history = InMemoryChatMessageHistory::new();
///
/// // Add messages
/// history.add_user_message(HumanMessage::builder().content("Hello!").build());
/// history.add_ai_message(AIMessage::builder().content("Hi there!").build());
///
/// // Get all messages
/// let messages = history.messages();
/// assert_eq!(messages.len(), 2);
///
/// // Clear history
/// history.clear();
/// assert!(history.messages().is_empty());
/// ```
#[async_trait]
pub trait BaseChatMessageHistory: Send + Sync {
    /// Get the list of messages.
    ///
    /// In general, getting the messages may involve IO to the underlying
    /// persistence layer, so this operation is expected to incur some
    /// latency.
    fn messages(&self) -> Vec<BaseMessage>;

    /// Async version of getting messages.
    ///
    /// Can override this method to provide an efficient async implementation.
    ///
    /// In general, fetching messages may involve IO to the underlying
    /// persistence layer.
    async fn get_messages_async(&self) -> Vec<BaseMessage> {
        self.messages()
    }

    /// Convenience method for adding a human message string to the store.
    ///
    /// Note: This is a convenience method. Code should favor the bulk `add_messages`
    /// interface instead to save on round-trips to the persistence layer.
    ///
    /// This method may be deprecated in a future release.
    fn add_user_message(&mut self, message: HumanMessage) {
        self.add_message(BaseMessage::Human(message));
    }

    /// Convenience method for adding an AI message string to the store.
    ///
    /// Note: This is a convenience method. Code should favor the bulk `add_messages`
    /// interface instead to save on round-trips to the persistence layer.
    ///
    /// This method may be deprecated in a future release.
    fn add_ai_message(&mut self, message: AIMessage) {
        self.add_message(BaseMessage::AI(message));
    }

    /// Add a Message object to the store.
    ///
    /// By default, this calls `add_messages` with a single-element slice.
    /// Implementations should override `add_messages` to provide efficient
    /// bulk addition.
    fn add_message(&mut self, message: BaseMessage) {
        self.add_messages(&[message]);
    }

    /// Add a list of messages.
    ///
    /// Implementations should override this method to handle bulk addition of messages
    /// in an efficient manner to avoid unnecessary round-trips to the underlying store.
    fn add_messages(&mut self, messages: &[BaseMessage]);

    /// Async add a list of messages.
    ///
    /// Default implementation calls the sync version.
    /// Override for truly async implementations.
    async fn add_messages_async(&mut self, messages: Vec<BaseMessage>) {
        self.add_messages(&messages);
    }

    /// Remove all messages from the store.
    fn clear(&mut self);

    /// Async remove all messages from the store.
    ///
    /// Default implementation calls the sync version.
    /// Override for truly async implementations.
    async fn clear_async(&mut self) {
        self.clear();
    }

    /// Return a string representation of the chat history.
    fn to_buffer_string(&self) -> String {
        get_buffer_string(&self.messages(), "Human", "AI")
    }
}

/// In memory implementation of chat message history.
///
/// Stores messages in a memory list.
#[derive(Debug, Clone, Default)]
pub struct InMemoryChatMessageHistory {
    /// A list of messages stored in memory.
    messages: Vec<BaseMessage>,
}

impl InMemoryChatMessageHistory {
    /// Create a new empty chat message history.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Create a chat message history with initial messages.
    pub fn with_messages(messages: Vec<BaseMessage>) -> Self {
        Self { messages }
    }
}

impl Display for InMemoryChatMessageHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_buffer_string())
    }
}

#[async_trait]
impl BaseChatMessageHistory for InMemoryChatMessageHistory {
    fn messages(&self) -> Vec<BaseMessage> {
        self.messages.clone()
    }

    async fn get_messages_async(&self) -> Vec<BaseMessage> {
        self.messages.clone()
    }

    fn add_messages(&mut self, messages: &[BaseMessage]) {
        self.messages.extend(messages.iter().cloned());
    }

    async fn add_messages_async(&mut self, messages: Vec<BaseMessage>) {
        self.add_messages(&messages);
    }

    fn clear(&mut self) {
        self.messages.clear();
    }

    async fn clear_async(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_chat_history_new() {
        let history = InMemoryChatMessageHistory::new();
        assert!(history.messages().is_empty());
    }

    #[test]
    fn test_in_memory_chat_history_with_messages() {
        let messages = vec![
            BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
            BaseMessage::AI(AIMessage::builder().content("Hi there!").build()),
        ];
        let history = InMemoryChatMessageHistory::with_messages(messages.clone());
        assert_eq!(history.messages().len(), 2);
    }

    #[test]
    fn test_add_user_message_string() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_user_message(HumanMessage::builder().content("Hello!").build());

        let messages = history.messages();
        assert_eq!(messages.len(), 1);
        assert!(matches!(&messages[0], BaseMessage::Human(_)));
        assert_eq!(messages[0].content(), "Hello!");
    }

    #[test]
    fn test_add_user_message_human_message() {
        let mut history = InMemoryChatMessageHistory::new();
        let human_msg = HumanMessage::builder().content("Hello!").build();
        history.add_user_message(human_msg);

        let messages = history.messages();
        assert_eq!(messages.len(), 1);
        assert!(matches!(&messages[0], BaseMessage::Human(_)));
        assert_eq!(messages[0].content(), "Hello!");
    }

    #[test]
    fn test_add_ai_message_string() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_ai_message(AIMessage::builder().content("Hi there!").build());

        let messages = history.messages();
        assert_eq!(messages.len(), 1);
        assert!(matches!(&messages[0], BaseMessage::AI(_)));
        assert_eq!(messages[0].content(), "Hi there!");
    }

    #[test]
    fn test_add_ai_message_ai_message() {
        let mut history = InMemoryChatMessageHistory::new();
        let ai_msg = AIMessage::builder().content("Hi there!").build();
        history.add_ai_message(ai_msg);

        let messages = history.messages();
        assert_eq!(messages.len(), 1);
        assert!(matches!(&messages[0], BaseMessage::AI(_)));
        assert_eq!(messages[0].content(), "Hi there!");
    }

    #[test]
    fn test_add_message() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_message(BaseMessage::Human(
            HumanMessage::builder().content("Hello").build(),
        ));
        history.add_message(BaseMessage::AI(AIMessage::builder().content("Hi").build()));

        let messages = history.messages();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_add_messages() {
        let mut history = InMemoryChatMessageHistory::new();
        let new_messages = vec![
            BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
            BaseMessage::AI(AIMessage::builder().content("Hi").build()),
            BaseMessage::Human(HumanMessage::builder().content("How are you?").build()),
        ];
        history.add_messages(&new_messages);

        let messages = history.messages();
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn test_clear() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_user_message(HumanMessage::builder().content("Hello!").build());
        history.add_ai_message(AIMessage::builder().content("Hi!").build());

        assert_eq!(history.messages().len(), 2);

        history.clear();
        assert!(history.messages().is_empty());
    }

    #[test]
    fn test_to_buffer_string() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_user_message(HumanMessage::builder().content("Hello!").build());
        history.add_ai_message(AIMessage::builder().content("Hi there!").build());

        let buffer = history.to_buffer_string();
        assert!(buffer.contains("Human: Hello!"));
        assert!(buffer.contains("AI: Hi there!"));
    }

    #[test]
    fn test_display() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_user_message(HumanMessage::builder().content("Hello!").build());
        history.add_ai_message(AIMessage::builder().content("Hi there!").build());

        let display = format!("{}", history);
        assert!(display.contains("Human: Hello!"));
        assert!(display.contains("AI: Hi there!"));
    }

    #[tokio::test]
    async fn test_get_messages_async() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_user_message(HumanMessage::builder().content("Hello!").build());

        let messages = history.get_messages_async().await;
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_add_messages_async() {
        let mut history = InMemoryChatMessageHistory::new();
        let new_messages = vec![
            BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
            BaseMessage::AI(AIMessage::builder().content("Hi").build()),
        ];
        history.add_messages_async(new_messages).await;

        let messages = history.messages();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_clear_async() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_user_message(HumanMessage::builder().content("Hello!").build());

        assert_eq!(history.messages().len(), 1);

        history.clear_async().await;
        assert!(history.messages().is_empty());
    }
}
