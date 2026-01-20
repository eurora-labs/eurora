mod error;
mod manager;
mod types;

pub use manager::ConversationManager;
pub use types::Conversation;

pub use proto_gen::conversation::ListConversationsRequest;
