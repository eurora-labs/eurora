use crate::shared_types::SharedConversationManager;
use agent_chain_core::BaseMessage;
use euro_conversation::{Conversation, ListConversationsRequest};
use tauri::{Manager, Runtime};
use tracing::error;

#[taurpc::ipc_type]
pub struct ConversationView {
    pub id: Option<String>,
    pub title: String,
}

#[taurpc::ipc_type]
pub struct MessageView {
    pub id: Option<String>,
    pub role: String,
    pub content: String,
}

#[taurpc::procedures(path = "conversation")]
pub trait ConversationApi {
    #[taurpc(event)]
    async fn new_conversation_added(conversation: ConversationView);

    #[taurpc(event)]
    async fn conversation_title_changed(conversation: ConversationView);

    #[taurpc(event)]
    async fn current_conversation_changed(conversation: ConversationView);

    async fn switch_conversation<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
    ) -> Result<ConversationView, String>;

    async fn list<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ConversationView>, String>;

    async fn create_empty_conversation<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ConversationView, String>;

    async fn create<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ConversationView, String>;

    async fn get_messages<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageView>, String>;
}

#[derive(Clone)]
pub struct ConversationApiImpl;

#[taurpc::resolvers]
impl ConversationApi for ConversationApiImpl {
    async fn list<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ConversationView>, String> {
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let conversation_manager = conversation_state.lock().await;

        let conversations = conversation_manager
            .list_conversations(ListConversationsRequest { limit, offset })
            .await
            .map_err(|e| e.to_string())?;

        Ok(conversations
            .into_iter()
            .map(|conversation| conversation.into())
            .collect())
    }

    async fn create_empty_conversation<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ConversationView, String> {
        let event_handler = app_handle.clone();
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let mut conversation_manager = conversation_state.lock().await;

        let conversation = conversation_manager
            .create_empty_conversation()
            .await
            .map_err(|e| e.to_string())?;

        let view: ConversationView = conversation.into();

        match TauRpcConversationApiEventTrigger::new(event_handler)
            .current_conversation_changed(view.clone())
        {
            Ok(_) => Ok(view),
            Err(e) => {
                error!(
                    "Failed to trigger current conversation changed event: {}",
                    e
                );
                Err(e.to_string())
            }
        }
    }

    async fn create<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ConversationView, String> {
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let _conversation_manager = conversation_state.lock().await;

        todo!()
    }

    async fn get_messages<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageView>, String> {
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let conversation_manager = conversation_state.lock().await;
        let messages = conversation_manager
            .get_messages(conversation_id, limit, offset)
            .await
            .map_err(|e| format!("Failed to get messages: {}", e))?;

        Ok(messages
            .into_iter()
            .filter_map(|message| match message {
                BaseMessage::System(_) => None,
                _ => Some(MessageView::from(message)),
            })
            .collect())
    }

    async fn switch_conversation<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
    ) -> Result<ConversationView, String> {
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let mut conversation_manager = conversation_state.lock().await;

        let conversation = conversation_manager
            .switch_conversation(conversation_id)
            .await
            .map_err(|e| format!("Failed to switch conversation: {}", e))?;

        TauRpcConversationApiEventTrigger::new(app_handle.clone())
            .current_conversation_changed(conversation.into())
            .map_err(|e| e.to_string())?;

        Ok(conversation.into())
    }
}

impl From<Conversation> for ConversationView {
    fn from(conversation: Conversation) -> Self {
        ConversationView {
            id: conversation.id().map(|id| id.to_string()),
            title: conversation.title().to_string(),
        }
    }
}

impl From<&Conversation> for ConversationView {
    fn from(conversation: &Conversation) -> Self {
        ConversationView {
            id: conversation.id().map(|id| id.to_string()),
            title: conversation.title().to_string(),
        }
    }
}

impl From<&BaseMessage> for MessageView {
    fn from(message: &BaseMessage) -> Self {
        MessageView {
            id: message.id(),
            role: message.message_type().to_string(),
            content: message.content().to_string(),
        }
    }
}

impl From<BaseMessage> for MessageView {
    fn from(message: BaseMessage) -> Self {
        MessageView {
            id: message.id(),
            role: message.message_type().to_string(),
            content: message.content().to_string(),
        }
    }
}
