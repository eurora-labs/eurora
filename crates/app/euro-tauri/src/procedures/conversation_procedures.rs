use crate::shared_types::SharedConversationManager;
use agent_chain_core::BaseMessage;
use euro_conversation::{Conversation, ListConversationsRequest};
use tauri::{Manager, Runtime};

#[taurpc::ipc_type]
pub struct ConversationView {
    pub id: Option<String>,
    pub title: String,
    pub messages: Vec<MessageView>,
}

#[taurpc::ipc_type]
pub struct MessageView {
    pub id: String,
    pub role: String,
    pub content: String,
}

#[taurpc::procedures(path = "conversation")]
pub trait ConversationApi {
    #[taurpc(event)]
    async fn new_conversation_added(conversation: ConversationView);

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

    async fn create<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ConversationView, String>;

    async fn get_messages<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
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

    async fn create<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ConversationView, String> {
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let _conversation_manager = conversation_state.lock().await;

        // Ok(conversation_manager
        //     .get_current_conversation()
        //     .await
        //     .clone());
        todo!()
        // let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();

        // let conversation = personal_db
        //     .insert_empty_conversation()
        //     .await
        //     .map_err(|e| e.to_string())?;

        // let current = app_handle.state::<SharedCurrentConversation>();
        // let mut guard = current.lock().await;
        // *guard = Some(conversation.clone());

        // TauRpcConversationApiEventTrigger::new(app_handle.clone())
        //     .new_conversation_added(conversation.clone())
        //     .map_err(|e| e.to_string())?;

        // Ok(conversation)
    }

    async fn get_messages<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
    ) -> Result<Vec<MessageView>, String> {
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let conversation_manager = conversation_state.lock().await;

        let conversation = conversation_manager
            .get_conversation(conversation_id)
            .await
            .map_err(|e| format!("Failed to get conversation: {}", e))?;

        Ok(conversation
            .messages()
            .iter()
            .map(MessageView::from)
            .collect())
    }

    async fn switch_conversation<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        _conversation_id: String,
    ) -> Result<ConversationView, String> {
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let _conversation_manager = conversation_state.lock().await;

        todo!("Implement switch_conversation")
        // let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();

        // let conversation = personal_db
        //     .get_conversation(&conversation_id)
        //     .await
        //     .map_err(|e| format!("Failed to get conversation: {}", e))?;

        // let current = app_handle.state::<SharedCurrentConversation>();
        // let mut guard = current.lock().await;
        // *guard = Some(conversation.clone());

        // TauRpcChatApiEventTrigger::new(app_handle.clone())
        //     .current_conversation_changed(conversation.clone())
        //     .map_err(|e| e.to_string())?;

        // let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        // let conversation_manager = conversation_state.lock().await;
        // Ok(conversation_manager
        //     .get_current_conversation()
        //     .await
        //     .clone())
    }
}

impl From<Conversation> for ConversationView {
    fn from(conversation: Conversation) -> Self {
        ConversationView {
            id: conversation.id().map(|id| id.to_string()),
            title: conversation.title().to_string(),
            messages: conversation
                .messages()
                .iter()
                .map(MessageView::from)
                .collect(),
        }
    }
}

impl From<&BaseMessage> for MessageView {
    fn from(message: &BaseMessage) -> Self {
        MessageView {
            id: message.id().unwrap_or_default(),
            role: message.message_type().to_string(),
            content: message.content().to_string(),
        }
    }
}
