use eur_settings::{HoverSettings, TelemetrySettings};
use tauri::{Manager, Runtime};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tracing::{error, info};

use crate::shared_types::SharedAppSettings;

#[taurpc::procedures(path = "settings")]
pub trait SettingsApi {
    async fn get_hover_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<HoverSettings, String>;

    async fn get_telemetry_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<TelemetrySettings, String>;
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
}
