use euro_settings::{AppSettings, GeneralSettings, TelemetrySettings, ThirdPartySettings};
use tauri::{Manager, Runtime};

use crate::shared_types::SharedAppSettings;

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

    async fn get_third_party_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ThirdPartySettings, String>;

    async fn set_third_party_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        third_party_settings: ThirdPartySettings,
    ) -> Result<ThirdPartySettings, String>;
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

    async fn get_third_party_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ThirdPartySettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.third_party.clone())
    }

    async fn set_third_party_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        third_party_settings: ThirdPartySettings,
    ) -> Result<ThirdPartySettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        settings.third_party = third_party_settings;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to persist third party settings: {e}"))?;

        Ok(settings.third_party.clone())
    }
}
