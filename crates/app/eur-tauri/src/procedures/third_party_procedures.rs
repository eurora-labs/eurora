use crate::shared_types::SharedPromptKitService;
use eur_secret::Sensitive;
use eur_secret::secret;
use tauri::{Manager, Runtime};
#[taurpc::procedures(path = "third_party")]
pub trait ThirdPartyApi {
    async fn switch_to_ollama<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        base_url: String,
        model: String,
    ) -> Result<(), String>;
    async fn check_api_key_exists() -> Result<bool, String>;
    async fn save_api_key(api_key: String) -> Result<(), String>;
    async fn initialize_openai_client<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<bool, String>;
}

#[derive(Clone)]
pub struct ThirdPartyApiImpl;

#[taurpc::resolvers]
impl ThirdPartyApi for ThirdPartyApiImpl {
    async fn check_api_key_exists(self) -> Result<bool, String> {
        let key = secret::retrieve("OPENAI_API_KEY", secret::Namespace::Global)
            .map_err(|e| format!("Failed to retrieve API key: {}", e))?;

        let key = key.map(|s| s.0);

        if key.is_none() {
            return Ok(false);
        }

        Ok(true)
    }

    async fn save_api_key(self, api_key: String) -> Result<(), String> {
        secret::persist(
            "OPENAI_API_KEY",
            &Sensitive(api_key),
            secret::Namespace::Global,
        )
        .map_err(|e| format!("Failed to save API key: {}", e))?;
        Ok(())
    }

    async fn initialize_openai_client<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<bool, String> {
        let api_key = secret::retrieve("OPENAI_API_KEY", secret::Namespace::Global)
            .map_err(|e| format!("Failed to retrieve API key: {}", e))?;

        // Initialize the OpenAI client with the API key
        let promptkit_client = eur_prompt_kit::PromptKitService::default();

        // Store the client in the app state
        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        *guard = Some(promptkit_client);

        Ok(true)
    }

    async fn switch_to_ollama<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        base_url: String,
        model: String,
    ) -> Result<(), String> {
        let mut promptkit_client = eur_prompt_kit::PromptKitService::default();
        promptkit_client
            .switch_to_ollama(eur_prompt_kit::OllamaConfig { base_url, model })
            .await?;
        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        *guard = Some(promptkit_client);
        Ok(())
    }
}
