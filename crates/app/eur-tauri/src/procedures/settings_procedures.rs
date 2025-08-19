use eur_settings::{GeneralSettings, HoverSettings, TelemetrySettings};
use tauri::{Manager, Runtime};

use crate::shared_types::SharedAppSettings;

#[taurpc::procedures(path = "settings")]
pub trait SettingsApi {
    async fn get_hover_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<HoverSettings, String>;

    async fn get_telemetry_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<TelemetrySettings, String>;

    async fn get_general_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<GeneralSettings, String>;
}
#[derive(Clone)]
pub struct SettingsApiImpl;

#[taurpc::resolvers]
impl SettingsApi for SettingsApiImpl {
    async fn get_hover_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<HoverSettings, String> {
        let settings: tauri::State<SharedAppSettings> = app_handle.state();
        let settings = settings.lock().await;
        Ok(settings.hover.clone())
    }

    async fn get_telemetry_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<TelemetrySettings, String> {
        let settings: tauri::State<SharedAppSettings> = app_handle.state();
        let settings = settings.lock().await;
        Ok(settings.telemetry.clone())
    }

    async fn get_general_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<GeneralSettings, String> {
        let settings: tauri::State<SharedAppSettings> = app_handle.state();
        let settings = settings.lock().await;
        Ok(settings.general.clone())
    }
}
