use agent_chain::{BaseMessage, convert_to_message};
use be_remote_db::{Message, MessageType};

use crate::{ConversationServiceError, ConversationServiceResult};

pub fn convert_db_message_to_base_message(
    db_message: Message,
) -> ConversationServiceResult<BaseMessage> {
    match db_message.message_type {
        MessageType::Human => {
            let mut message = convert_to_message(&db_message.content).map_err(|e| {
                ConversationServiceError::Internal(format!("Failed to convert message: {}", e))
            })?;
            message.set_id(db_message.id.to_string());
            Ok(message)
        }
        _ => todo!(),
    }
}
