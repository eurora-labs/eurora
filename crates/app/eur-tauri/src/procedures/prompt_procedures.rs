use crate::shared_types::SharedPromptKitService;
use tauri::{Manager, Runtime};

#[taurpc::procedures(path = "prompt")]
pub trait PromptApi {
    #[taurpc(event)]
    async fn prompt_service_change(service_name: Option<String>);

    async fn switch_to_ollama<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        base_url: String,
        model: String,
    ) -> Result<(), String>;
    async fn switch_to_remote<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        provider: String,
        api_key: String,
        model: String,
    ) -> Result<(), String>;
    async fn get_service_name<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String>;

    async fn disconnect<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;
}

#[derive(Clone)]
pub struct PromptApiImpl;

#[taurpc::resolvers]
impl PromptApi for PromptApiImpl {
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

        TauRpcPromptApiEventTrigger::new(app_handle.clone())
            .prompt_service_change(Some(
                promptkit_client
                    .get_service_name()
                    .map_err(|e| e.to_string())?,
            ))
            .map_err(|e| e.to_string())?;

        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        *guard = Some(promptkit_client);

        Ok(())
    }

    async fn switch_to_remote<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        provider: String,
        api_key: String,
        model: String,
    ) -> Result<(), String> {
        let mut promptkit_client = eur_prompt_kit::PromptKitService::default();
        promptkit_client
            .switch_to_remote(eur_prompt_kit::RemoteConfig {
                provider: provider.into(),
                api_key,
                model,
            })
            .await?;

        TauRpcPromptApiEventTrigger::new(app_handle.clone())
            .prompt_service_change(Some(
                promptkit_client
                    .get_service_name()
                    .map_err(|e| e.to_string())?,
            ))
            .map_err(|e| e.to_string())?;

        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        *guard = Some(promptkit_client);

        Ok(())
    }

    async fn get_service_name<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String> {
        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let guard = state.lock().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| "PromptKitService not initialized".to_string())?;
        client.get_service_name().map_err(|e| e.to_string())
    }

    async fn disconnect<R: Runtime>(self, app_handle: tauri::AppHandle<R>) -> Result<(), String> {
        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        *guard = None;
        TauRpcPromptApiEventTrigger::new(app_handle.clone())
            .prompt_service_change(None)
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
