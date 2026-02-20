use async_trait::async_trait;
use std::fmt::Display;

use crate::messages::{AIMessage, BaseMessage, HumanMessage, get_buffer_string};

#[async_trait]
pub trait BaseChatMessageHistory: Send + Sync {
    fn messages(&self) -> Vec<BaseMessage>;

    async fn get_messages_async(&self) -> Vec<BaseMessage> {
        self.messages()
    }

    fn add_user_message(&mut self, message: HumanMessage) {
        self.add_message(BaseMessage::Human(message));
    }

    fn add_ai_message(&mut self, message: AIMessage) {
        self.add_message(BaseMessage::AI(message));
    }

    fn add_message(&mut self, message: BaseMessage) {
        self.add_messages(&[message]);
    }

    fn add_messages(&mut self, messages: &[BaseMessage]);

    async fn add_messages_async(&mut self, messages: Vec<BaseMessage>) {
        self.add_messages(&messages);
    }

    fn clear(&mut self);

    async fn clear_async(&mut self) {
        self.clear();
    }

    fn to_buffer_string(&self) -> String {
        get_buffer_string(&self.messages(), "Human", "AI")
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryChatMessageHistory {
    messages: Vec<BaseMessage>,
}

impl InMemoryChatMessageHistory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

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
