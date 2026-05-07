use std::sync::Arc;

use euro_settings::{
    APISettings, AppSettings, AppearanceSettings, GeneralSettings, TelemetrySettings,
};
use tauri::{Manager, Runtime};

use crate::error::ResultExt;
use crate::shared_types::{SharedAppSettings, SharedEndpointManager};
use crate::telemetry;

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
    ) -> Result<APISettings, String>;

    async fn set_api_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        api_settings: APISettings,
    ) -> Result<APISettings, String>;

    async fn get_appearance_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<AppearanceSettings, String>;

    async fn set_appearance_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        appearance_settings: AppearanceSettings,
    ) -> Result<AppearanceSettings, String>;
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
            .ctx("Failed to persist general settings")?;

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
        // Any save through this procedure is by definition a recorded
        // consent at the current schema version. Stamping it here
        // (rather than trusting the frontend) prevents an old client
        // from accidentally pinning the user at a stale version.
        settings.telemetry.record_consent();
        // Allocate a stable id alongside the first consent so that
        // an exit between this call and the next save doesn't drop
        // the id.
        settings.telemetry.ensure_distinct_id();
        settings
            .save_to_default_path()
            .ctx("Failed to persist telemetry settings")?;

        let new_telemetry = settings.telemetry.clone();
        drop(settings);

        if let Some(controller) = app_handle.try_state::<Arc<telemetry::Controller>>() {
            controller.reapply(&new_telemetry);
        }

        Ok(new_telemetry)
    }

    async fn get_api_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<APISettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.api.clone())
    }

    async fn set_api_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        api_settings: APISettings,
    ) -> Result<APISettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        settings.api = api_settings;

        settings
            .save_to_default_path()
            .ctx("Failed to persist api settings")?;

        // Translate the persisted connection mode into a concrete URL and
        // hand it to the shared endpoint manager so in-flight requests pick
        // up the change without a restart.
        let endpoint = settings.api.endpoint().to_string();
        let endpoint_manager = app_handle.state::<SharedEndpointManager>();
        endpoint_manager
            .set_global_backend_url(&endpoint)
            .ctx("Failed to switch API endpoint")?;

        Ok(settings.api.clone())
    }

    async fn get_appearance_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<AppearanceSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.appearance.clone())
    }

    async fn set_appearance_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        appearance_settings: AppearanceSettings,
    ) -> Result<AppearanceSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        settings.appearance = appearance_settings;
        settings
            .save_to_default_path()
            .ctx("Failed to persist appearance settings")?;

        Ok(settings.appearance.clone())
    }
}
