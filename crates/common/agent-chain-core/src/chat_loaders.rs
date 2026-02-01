//! **Chat loaders** are used to load chat sessions from various sources.
//!
//! This module provides the [`BaseChatLoader`] trait for implementing chat
//! session loaders that can lazily or eagerly load chat sessions.
//!
//! Mirrors `langchain_core.chat_loaders`.

use crate::chat_sessions::ChatSession;

/// Base trait for chat loaders.
///
/// Chat loaders are responsible for loading chat sessions from various sources
/// (files, databases, APIs, etc.). Implementations must provide a [`lazy_load`](BaseChatLoader::lazy_load)
/// method that returns an iterator of chat sessions.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::chat_loaders::BaseChatLoader;
/// use agent_chain_core::chat_sessions::ChatSession;
/// use agent_chain_core::messages::{BaseMessage, HumanMessage, AIMessage};
///
/// struct MyLoader {
///     sessions: Vec<ChatSession>,
/// }
///
/// impl BaseChatLoader for MyLoader {
///     fn lazy_load(&self) -> Box<dyn Iterator<Item = ChatSession> + '_> {
///         Box::new(self.sessions.iter().cloned())
///     }
/// }
///
/// let loader = MyLoader {
///     sessions: vec![
///         ChatSession::with_messages(vec![
///             BaseMessage::Human(HumanMessage::new("Hello")),
///             BaseMessage::AI(AIMessage::builder().content("Hi there!").build()),
///         ]),
///     ],
/// };
///
/// // Lazy iteration
/// for session in loader.lazy_load() {
///     println!("Session has {} messages", session.messages().len());
/// }
///
/// // Eager loading
/// let all_sessions = loader.load();
/// println!("Loaded {} sessions", all_sessions.len());
/// ```
pub trait BaseChatLoader {
    /// Lazy load the chat sessions.
    ///
    /// Returns an iterator of chat sessions. This allows for memory-efficient
    /// processing of large datasets where not all sessions need to be loaded
    /// into memory at once.
    ///
    /// # Returns
    ///
    /// An iterator of chat sessions.
    fn lazy_load(&self) -> Box<dyn Iterator<Item = ChatSession> + '_>;

    /// Eagerly load the chat sessions into memory.
    ///
    /// This is a convenience method that collects all chat sessions from
    /// [`lazy_load`](BaseChatLoader::lazy_load) into a vector.
    ///
    /// # Returns
    ///
    /// A vector of chat sessions.
    fn load(&self) -> Vec<ChatSession> {
        self.lazy_load().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{AIMessage, BaseMessage, HumanMessage};

    /// A simple in-memory chat loader for testing.
    struct InMemoryChatLoader {
        sessions: Vec<ChatSession>,
    }

    impl InMemoryChatLoader {
        fn new(sessions: Vec<ChatSession>) -> Self {
            Self { sessions }
        }
    }

    impl BaseChatLoader for InMemoryChatLoader {
        fn lazy_load(&self) -> Box<dyn Iterator<Item = ChatSession> + '_> {
            Box::new(self.sessions.iter().cloned())
        }
    }

    #[test]
    fn test_lazy_load() {
        let sessions = vec![
            ChatSession::with_messages(vec![
                BaseMessage::Human(HumanMessage::new("Hello")),
                BaseMessage::AI(AIMessage::builder().content("Hi").build()),
            ]),
            ChatSession::with_messages(vec![BaseMessage::Human(HumanMessage::new("Bye"))]),
        ];

        let loader = InMemoryChatLoader::new(sessions);

        let mut count = 0;
        for session in loader.lazy_load() {
            assert!(session.has_messages());
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_load() {
        let sessions = vec![
            ChatSession::with_messages(vec![BaseMessage::Human(HumanMessage::new("Hello"))]),
            ChatSession::with_messages(vec![BaseMessage::AI(AIMessage::builder().content("Hi").build())]),
            ChatSession::with_messages(vec![BaseMessage::Human(HumanMessage::new("Bye"))]),
        ];

        let loader = InMemoryChatLoader::new(sessions);
        let loaded = loader.load();

        assert_eq!(loaded.len(), 3);
    }

    #[test]
    fn test_empty_loader() {
        let loader = InMemoryChatLoader::new(vec![]);

        assert!(loader.load().is_empty());
        assert_eq!(loader.lazy_load().count(), 0);
    }
}
