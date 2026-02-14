use euro_settings::{ApiSettings, AppSettings, GeneralSettings, TelemetrySettings};
use tauri::{Manager, Runtime};

use crate::shared_types::{SharedAppSettings, SharedEndpointManager};

#[taurpc::procedures(path = "settings")]
pub trait SettingsApi {
    async fn get_all_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<AppSettings, String>;

    async fn get_telemetry_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<TelemetrySettings, String>;

    async fn get_general_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<GeneralSettings, String>;

    async fn set_general_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        general_settings: GeneralSettings,
    ) -> Result<GeneralSettings, String>;

    async fn set_telemetry_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        telemetry_settings: TelemetrySettings,
    ) -> Result<TelemetrySettings, String>;

    async fn get_api_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ApiSettings, String>;

    async fn set_api_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        api_settings: ApiSettings,
    ) -> Result<ApiSettings, String>;
}
#[derive(Clone)]
pub struct SettingsApiImpl;

#[taurpc::resolvers]
impl SettingsApi for SettingsApiImpl {
    async fn get_all_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<AppSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.clone())
    }

    async fn get_telemetry_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<TelemetrySettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.telemetry.clone())
    }

    async fn get_general_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<GeneralSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.general.clone())
    }

    async fn set_general_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        general_settings: GeneralSettings,
    ) -> Result<GeneralSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        settings.general = general_settings;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to persist general settings: {e}"))?;

        Ok(settings.general.clone())
    }

    async fn set_telemetry_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        telemetry_settings: TelemetrySettings,
    ) -> Result<TelemetrySettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        settings.telemetry = telemetry_settings;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to persist telemetry settings: {e}"))?;

        Ok(settings.telemetry.clone())
    }

    async fn get_api_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ApiSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.api.clone())
    }

    async fn set_api_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        api_settings: ApiSettings,
    ) -> Result<ApiSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        let new_endpoint = api_settings.endpoint.clone();
        settings.api = api_settings;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to persist api settings: {e}"))?;

        if !new_endpoint.is_empty() {
            let endpoint_manager = app_handle.state::<SharedEndpointManager>();
            endpoint_manager
                .set_global_backend_url(&new_endpoint)
                .map_err(|e| format!("Failed to switch API endpoint: {e}"))?;
        }

        Ok(settings.api.clone())
    }
}
