use crate::{db::PersonalDatabaseManager, types::ChatMessage};
use chrono::Utc;
use ferrous_llm_core::{Message, MessageContent};

impl PersonalDatabaseManager {
    pub async fn insert_chat_message_from_message(
        &self,
        conversation_id: &str,
        message: Message,
    ) -> Result<ChatMessage, sqlx::Error> {
        let timestamp = Utc::now();

        // TODO: Implement other cases
        let content = match message.content {
            MessageContent::Text(message) => Some(message),
            _ => None,
        };

        if content.is_none() {
            return Err(sqlx::Error::InvalidArgument(
                "Content type is not implemented".to_string(),
            ));
        }

        self.insert_chat_message(
            conversation_id,
            message.role.to_string().as_str(),
            content.unwrap().as_str(),
            true,
            timestamp,
            timestamp,
        )
        .await
    }
}
