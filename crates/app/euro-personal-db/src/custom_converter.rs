use agent_chain::BaseMessage;
use chrono::Utc;

use crate::{NewChatMessage, db::PersonalDatabaseManager, types::ChatMessage};

impl PersonalDatabaseManager {
    pub async fn insert_chat_message_from_message(
        &self,
        conversation_id: &str,
        message: BaseMessage,
        has_assets: bool,
    ) -> Result<ChatMessage, sqlx::Error> {
        let timestamp = Utc::now();

        let content = message.content().to_string();
        let role = message.message_type().to_string();

        self.insert_chat_message(NewChatMessage {
            conversation_id: conversation_id.to_string(),
            role,
            content,
            has_assets,
            created_at: Some(timestamp),
            updated_at: Some(timestamp),
        })
        .await
    }
}
