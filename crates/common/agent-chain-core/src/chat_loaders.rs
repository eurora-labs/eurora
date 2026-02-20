use crate::chat_sessions::ChatSession;

pub trait BaseChatLoader {
    fn lazy_load(&self) -> Box<dyn Iterator<Item = ChatSession> + '_>;

    fn load(&self) -> Vec<ChatSession> {
        self.lazy_load().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{AIMessage, BaseMessage, HumanMessage};

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
                BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
                BaseMessage::AI(AIMessage::builder().content("Hi").build()),
            ]),
            ChatSession::with_messages(vec![BaseMessage::Human(
                HumanMessage::builder().content("Bye").build(),
            )]),
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
            ChatSession::with_messages(vec![BaseMessage::Human(
                HumanMessage::builder().content("Hello").build(),
            )]),
            ChatSession::with_messages(vec![BaseMessage::AI(
                AIMessage::builder().content("Hi").build(),
            )]),
            ChatSession::with_messages(vec![BaseMessage::Human(
                HumanMessage::builder().content("Bye").build(),
            )]),
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
