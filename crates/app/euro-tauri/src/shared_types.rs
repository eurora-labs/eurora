use std::sync::Arc;

use euro_conversation::ConversationManager;
use euro_endpoint::EndpointManager;
use euro_settings::AppSettings;
use tokio::sync::Mutex;

pub type SharedAppSettings = Mutex<AppSettings>;
pub type SharedConversationManager = Mutex<ConversationManager>;
pub type SharedEndpointManager = Arc<EndpointManager>;
pub type SharedUserController = Mutex<euro_user::Controller>;
