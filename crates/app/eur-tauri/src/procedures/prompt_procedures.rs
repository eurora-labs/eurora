use async_from::AsyncTryFrom;
use eur_eurora_provider::EuroraConfig;
use eur_prompt_kit::{OllamaConfig, OpenAIConfig};
use eur_secret::secret;
use eur_settings::BackendType;
use tauri::{Manager, Runtime};
use url::Url;

use crate::shared_types::{SharedAppSettings, SharedPromptKitService};

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
        let config = OllamaConfig::builder()
            .model(model)
            .base_url(base_url)
            .expect("Failed to connect to Ollama")
            .keep_alive(300)
            .build();

        let state = app_handle.state::<SharedAppSettings>();
        let mut app_settings = state.lock().await;
        app_settings.backend = config.clone().into();
        app_settings
            .save_to_default_path()
            .expect("Failed to save settings");

        let llm_provider = eur_prompt_kit::PromptKitService::from(config);

        TauRpcPromptApiEventTrigger::new(app_handle.clone())
            .prompt_service_change(Some(
                llm_provider.get_service_name().map_err(|e| e.to_string())?,
            ))
            .map_err(|e| e.to_string())?;

        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        *guard = Some(llm_provider);

        Ok(())
    }

    async fn switch_to_remote<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        _provider: String,
        api_key: String,
        model: String,
    ) -> Result<(), String> {
        let config = OpenAIConfig::builder()
            .api_key(api_key.clone())
            .model(model)
            .build();

        eur_secret::secret::persist(
            "OPENAI_API_KEY",
            &eur_secret::Sensitive(api_key.clone()),
            eur_secret::secret::Namespace::Global,
        )
        .map_err(|e| e.to_string())?;

        let state = app_handle.state::<SharedAppSettings>();
        let mut app_settings = state.lock().await;
        app_settings.backend = config.clone().into();
        app_settings
            .save_to_default_path()
            .expect("Failed to save settings");

        let llm_provider = eur_prompt_kit::PromptKitService::from(config);

        TauRpcPromptApiEventTrigger::new(app_handle.clone())
            .prompt_service_change(Some(
                llm_provider.get_service_name().map_err(|e| e.to_string())?,
            ))
            .map_err(|e| e.to_string())?;

        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        *guard = Some(llm_provider);

        Ok(())
    }

    async fn get_service_name<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String> {
        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;

        let client = guard.as_ref();
        if let Some(client) = client {
            Ok(client.get_service_name().map_err(|e| e.to_string())?)
        } else {
            let app_settings_state = app_handle.state::<SharedAppSettings>();
            let app_settings = app_settings_state.lock().await;
            let promptkit_client = match app_settings.backend.backend_type {
                BackendType::Eurora => {
                    secret::retrieve(eur_user::REFRESH_TOKEN_HANDLE, secret::Namespace::Global)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| "Refresh token not found".to_string())?;

                    // Initialize prompt kit
                    let config = EuroraConfig::new(
                        Url::parse(
                            std::env::var("API_BASE_URL")
                                .unwrap_or("https://api.eurora-labs.com".to_string())
                                .as_str(),
                        )
                        .map_err(|e| format!("Invalid API_BASE_URL: {}", e))?,
                    );

                    eur_prompt_kit::PromptKitService::async_try_from(config)
                        .await
                        .map_err(|e| e.to_string())?
                }
                BackendType::Ollama => app_settings
                    .backend
                    .initialize()
                    .await
                    .expect("Failed to initialize Ollama backend"),
                _ => todo!(),
            };

            let service_name = promptkit_client
                .get_service_name()
                .map_err(|e| e.to_string())?;

            TauRpcPromptApiEventTrigger::new(app_handle.clone())
                .prompt_service_change(Some(service_name.clone()))
                .map_err(|e| e.to_string())?;

            *guard = Some(promptkit_client);

            Ok(service_name)
        }
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
