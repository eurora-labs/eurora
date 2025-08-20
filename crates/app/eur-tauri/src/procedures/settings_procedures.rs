use crate::shared_types::SharedAppSettings;
use eur_settings::{GeneralSettings, HoverSettings, LauncherSettings, TelemetrySettings};
use tauri::{Manager, Runtime};

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
        settings.save_to_default_path().unwrap();

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
        settings.save_to_default_path().unwrap();

        Ok(())
    }

    async fn set_launcher_settings<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        launcher_settings: LauncherSettings,
    ) -> Result<(), String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        settings.launcher = launcher_settings;
        settings.save_to_default_path().unwrap();

        Ok(())
    }
}
