use crate::shared_types::SharedOpenAIClient;
use eur_secret::Sensitive;
use eur_secret::secret;
use tauri::{Manager, Runtime};
#[taurpc::procedures(path = "third_party")]
pub trait ThirdPartyApi {
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
        let key = secret::retrieve("OPEN_AI_API_KEY", secret::Namespace::Global)
            .map_err(|e| format!("Failed to retrieve API key: {}", e))?;

        let key = key.map(|s| s.0);

        if key.is_none() {
            return Ok(false);
        }

        Ok(true)
    }

    async fn save_api_key(self, api_key: String) -> Result<(), String> {
        secret::persist(
            "OPEN_AI_API_KEY",
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
        let api_key = secret::retrieve("OPEN_AI_API_KEY", secret::Namespace::Global)
            .map_err(|e| format!("Failed to retrieve API key: {}", e))?;

        // Initialize the OpenAI client with the API key
        let openai_client = eur_openai::OpenAI::with_api_key(&api_key.unwrap().0);

        // Store the client in the app state
        let state: tauri::State<SharedOpenAIClient> = app_handle.state();
        let mut guard = state.lock().await;
        *guard = Some(openai_client);

        Ok(true)
    }
}
