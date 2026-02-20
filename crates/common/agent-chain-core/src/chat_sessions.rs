use serde::{Deserialize, Serialize};

use crate::messages::BaseMessage;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatSession {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<BaseMessage>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<serde_json::Value>>,
}

impl ChatSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_messages(messages: Vec<BaseMessage>) -> Self {
        Self {
            messages: Some(messages),
            functions: None,
        }
    }

    pub fn with_messages_and_functions(
        messages: Vec<BaseMessage>,
        functions: Vec<serde_json::Value>,
    ) -> Self {
        Self {
            messages: Some(messages),
            functions: Some(functions),
        }
    }

    pub fn messages(&self) -> &[BaseMessage] {
        self.messages.as_deref().unwrap_or(&[])
    }

    pub fn functions(&self) -> &[serde_json::Value] {
        self.functions.as_deref().unwrap_or(&[])
    }

    pub fn has_messages(&self) -> bool {
        self.messages.as_ref().is_some_and(|m| !m.is_empty())
    }

    pub fn has_functions(&self) -> bool {
        self.functions.as_ref().is_some_and(|f| !f.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{AIMessage, HumanMessage};

    #[test]
    fn test_chat_session_new() {
        let session = ChatSession::new();
        assert!(session.messages.is_none());
        assert!(session.functions.is_none());
        assert!(!session.has_messages());
        assert!(!session.has_functions());
    }

    #[test]
    fn test_chat_session_with_messages() {
        let messages = vec![
            BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
            BaseMessage::AI(AIMessage::builder().content("Hi").build()),
        ];
        let session = ChatSession::with_messages(messages);

        assert!(session.has_messages());
        assert!(!session.has_functions());
        assert_eq!(session.messages().len(), 2);
    }

    #[test]
    fn test_chat_session_with_messages_and_functions() {
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder().content("Hello").build(),
        )];
        let functions = vec![serde_json::json!({
            "name": "get_weather",
            "parameters": {}
        })];

        let session = ChatSession::with_messages_and_functions(messages, functions);

        assert!(session.has_messages());
        assert!(session.has_functions());
        assert_eq!(session.messages().len(), 1);
        assert_eq!(session.functions().len(), 1);
    }

    #[test]
    fn test_chat_session_messages_accessor() {
        let session = ChatSession::new();
        assert!(session.messages().is_empty());

        let session_with_messages = ChatSession::with_messages(vec![BaseMessage::Human(
            HumanMessage::builder().content("Hello").build(),
        )]);
        assert_eq!(session_with_messages.messages().len(), 1);
    }

    #[test]
    fn test_chat_session_functions_accessor() {
        let session = ChatSession::new();
        assert!(session.functions().is_empty());

        let session_with_functions = ChatSession {
            messages: None,
            functions: Some(vec![serde_json::json!({"name": "test"})]),
        };
        assert_eq!(session_with_functions.functions().len(), 1);
    }

    #[test]
    fn test_chat_session_serialization() {
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder().content("Hello").build(),
        )];
        let session = ChatSession::with_messages(messages);

        let serialized = serde_json::to_string(&session).expect("serialization should work");
        assert!(serialized.contains("messages"));

        let deserialized: ChatSession =
            serde_json::from_str(&serialized).expect("deserialization should work");
        assert!(deserialized.has_messages());
    }
}
