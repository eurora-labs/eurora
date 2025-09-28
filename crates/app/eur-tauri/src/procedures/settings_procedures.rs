use eur_settings::{
    AppSettings, GeneralSettings, HoverSettings, LauncherSettings, TelemetrySettings,
};
use tauri::{Manager, Runtime};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tracing::debug;

use crate::{shared_types::SharedAppSettings, util::convert_hotkey_to_shortcut};

#[taurpc::procedures(path = "settings")]
pub trait SettingsApi {
    async fn get_all_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<AppSettings, String>;

    async fn get_hover_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<HoverSettings, String>;

    async fn get_telemetry_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<TelemetrySettings, String>;

    async fn get_general_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<GeneralSettings, String>;

    async fn get_launcher_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<LauncherSettings, String>;

    async fn set_general_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        general_settings: GeneralSettings,
    ) -> Result<(), String>;

    async fn set_hover_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        hover_settings: HoverSettings,
    ) -> Result<(), String>;

    async fn set_launcher_settings<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        launcher_settings: LauncherSettings,
    ) -> Result<(), String>;
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

    async fn get_hover_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<HoverSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.hover.clone())
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

    async fn get_launcher_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<LauncherSettings, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;

        Ok(settings.launcher.clone())
    }

    async fn set_general_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        general_settings: GeneralSettings,
    ) -> Result<(), String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        settings.general = general_settings;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to persist hover settings: {e}"))?;

        Ok(())
    }

    async fn set_hover_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        hover_settings: HoverSettings,
    ) -> Result<(), String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        settings.hover = hover_settings;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to persist hover settings: {e}"))?;

        Ok(())
    }

    async fn set_launcher_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        launcher_settings: LauncherSettings,
    ) -> Result<(), String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;
        debug!("Launcher settings changed: {:?}", launcher_settings);

        if settings.launcher.hotkey != launcher_settings.hotkey {
            let previous_hotkey = convert_hotkey_to_shortcut(settings.launcher.hotkey.clone());
            let new_hotkey = convert_hotkey_to_shortcut(launcher_settings.hotkey.clone());

            app_handle
                .global_shortcut()
                .unregister(previous_hotkey)
                .map_err(|e| {
                    format!(
                        "Failed to unregister previous shortcut '{}': {}",
                        previous_hotkey, e
                    )
                })?;

            app_handle
                .global_shortcut()
                .register(new_hotkey)
                .map_err(|e| format!("Failed to register new shortcut '{}': {}", new_hotkey, e))?;
        }

        settings.launcher = launcher_settings;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to persist launcher settings: {e}"))?;

        Ok(())
    }
}
