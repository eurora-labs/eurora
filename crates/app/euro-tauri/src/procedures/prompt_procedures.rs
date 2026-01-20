use tauri::Runtime;

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
        _app_handle: tauri::AppHandle<R>,
        _base_url: String,
        _model: String,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn switch_to_remote<R: Runtime>(
        self,
        _app_handle: tauri::AppHandle<R>,
        _provider: String,
        _api_key: String,
        _model: String,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn get_service_name<R: Runtime>(
        self,
        _app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String> {
        Ok("PLACEHOLDER".to_string())
    }

    async fn disconnect<R: Runtime>(self, _app_handle: tauri::AppHandle<R>) -> Result<(), String> {
        Ok(())
    }
}
