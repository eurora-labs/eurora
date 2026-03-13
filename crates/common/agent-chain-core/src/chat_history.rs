use async_trait::async_trait;
use std::fmt::Display;

use crate::messages::{AIMessage, AnyMessage, HumanMessage, get_buffer_string};
pub use crate::runnables::run_in_executor;

#[async_trait]
pub trait BaseChatMessageHistory: Send + Sync {
    fn messages(&self) -> Vec<AnyMessage>;

    fn add_user_message(&mut self, message: HumanMessage) {
        self.add_message(AnyMessage::HumanMessage(message));
    }

    fn add_ai_message(&mut self, message: AIMessage) {
        self.add_message(AnyMessage::AIMessage(message));
    }

    fn add_message(&mut self, message: AnyMessage) {
        self.add_messages(&[message]);
    }

    fn add_messages(&mut self, messages: &[AnyMessage]);

    async fn clear(&mut self);
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryChatMessageHistory {
    messages: Vec<AnyMessage>,
}

impl InMemoryChatMessageHistory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn with_messages(messages: Vec<AnyMessage>) -> Self {
        Self { messages }
    }
}

impl Display for InMemoryChatMessageHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", get_buffer_string(&self.messages, "Human", "AI"))
    }
}

#[async_trait]
impl BaseChatMessageHistory for InMemoryChatMessageHistory {
    fn messages(&self) -> Vec<AnyMessage> {
        self.messages.clone()
    }

    fn add_messages(&mut self, messages: &[AnyMessage]) {
        self.messages.extend(messages.iter().cloned());
    }

    async fn clear(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::prelude::*;

    #[tokio::test]
    async fn test_in_memory_chat_history_new() {
        let history = InMemoryChatMessageHistory::new();
        assert!(history.messages().is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_chat_history_with_messages() {
        let messages = vec![
            AnyMessage::HumanMessage(HumanMessage::builder().content("Hello").build()),
            AnyMessage::AIMessage(AIMessage::builder().content("Hi there!").build()),
        ];
        let history = InMemoryChatMessageHistory::with_messages(messages.clone());
        assert_eq!(history.messages().len(), 2);
    }

    #[tokio::test]
    async fn test_add_user_message_string() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_user_message(HumanMessage::builder().content("Hello!").build());

        let messages = history.messages();
        assert_eq!(messages.len(), 1);
        assert!(matches!(&messages[0], AnyMessage::HumanMessage(_)));
        assert_eq!(messages[0].content(), "Hello!");
    }

    #[tokio::test]
    async fn test_add_ai_message() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_ai_message(AIMessage::builder().content("Hi there!").build());

        let messages = history.messages();
        assert_eq!(messages.len(), 1);
        assert!(matches!(&messages[0], AnyMessage::AIMessage(_)));
        assert_eq!(messages[0].content(), "Hi there!");
    }

    #[tokio::test]
    async fn test_add_message() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_message(AnyMessage::HumanMessage(
            HumanMessage::builder().content("Hello").build(),
        ));
        history.add_message(AnyMessage::AIMessage(
            AIMessage::builder().content("Hi").build(),
        ));

        let messages = history.messages();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_add_messages() {
        let mut history = InMemoryChatMessageHistory::new();
        let new_messages = vec![
            AnyMessage::HumanMessage(HumanMessage::builder().content("Hello").build()),
            AnyMessage::AIMessage(AIMessage::builder().content("Hi").build()),
            AnyMessage::HumanMessage(HumanMessage::builder().content("How are you?").build()),
        ];
        history.add_messages(&new_messages);

        let messages = history.messages();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_clear() {
        let mut history = InMemoryChatMessageHistory::new();
        history.add_user_message(HumanMessage::builder().content("Hello!").build());
        history.add_ai_message(AIMessage::builder().content("Hi!").build());

        assert_eq!(history.messages().len(), 2);

        history.clear().await;
        assert!(history.messages().is_empty());
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
}
