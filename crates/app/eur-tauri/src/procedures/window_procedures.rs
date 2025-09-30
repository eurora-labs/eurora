use tauri::{Manager, Runtime};
use tracing::debug;

use crate::launcher::toggle_launcher_window as toggle_launcher;

#[taurpc::ipc_type]
pub struct LauncherInfo {
    pub background_image: Option<String>,
    pub monitor_id: String,
    pub launcher_x: i32,
    pub launcher_y: i32,
    pub launcher_width: u32,
    pub launcher_height: u32,
    pub monitor_width: u32,
    pub monitor_height: u32,
    pub monitor_x: i32,
    pub monitor_y: i32,
    pub capture_x: i32,
    pub capture_y: i32,
    pub monitor_scale_factor: f64,
}

#[taurpc::procedures(path = "window")]
pub trait WindowApi {
    #[taurpc(event)]
    async fn launcher_opened(info: LauncherInfo);

    #[taurpc(event)]
    async fn launcher_closed();

    #[taurpc(event)]
    async fn background_image_changed(base64: String);

    async fn get_scale_factor<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        height: f64,
    ) -> Result<f64, String>;

    async fn resize_launcher_window<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<(), String>;

    async fn open_launcher_window<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String>;

    async fn open_main_window<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;

    async fn hide_hover_window<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;

    async fn show_hover_window<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;
}

#[derive(Clone)]
pub struct WindowApiImpl;

#[taurpc::resolvers]
impl WindowApi for WindowApiImpl {
    async fn get_scale_factor<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        height: f64,
    ) -> Result<f64, String> {
        let window = app_handle.get_window("launcher").unwrap();

        #[cfg(windows)]
        {
            Ok(window.scale_factor().unwrap_or(1.0))
        }
        #[cfg(not(windows))]
        {
            let current_size = window.inner_size().unwrap();
            let scale_factor = (current_size.height as f64) / (height);
            Ok(scale_factor)
        }
    }

    async fn resize_launcher_window<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<(), String> {
        debug!(
            "resize_launcher_window: height: {}, scale_factor: {}",
            height, scale_factor
        );
        let window = app_handle.get_window("launcher").unwrap();
        // let new_height = height as f64 * scale_factor;
        // let new_width = width as f64 * scale_factor;

        let _ = window.set_size(
            tauri::Size::Physical(tauri::PhysicalSize { width, height })
                .to_logical::<f64>(1.0 / scale_factor),
        );
        Ok(())
    }

    async fn open_launcher_window<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        let window = app_handle
            .get_window("launcher")
            .ok_or_else(|| "Launcher window not found".to_string())?;

        toggle_launcher(&window).map_err(|e| format!("Failed to open launcher window: {e}"))?;
        Ok(())
    }

    async fn open_main_window<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        let main_window = app_handle
            .get_window("main")
            .ok_or_else(|| "Main window not found".to_string())?;

        main_window
            .unminimize()
            .map_err(|e| format!("Failed to unminimize main window: {e}"))?;

        main_window
            .show()
            .map_err(|e| format!("Failed to show main window: {e}"))?;

        let launcher_window = app_handle
            .get_window("launcher")
            .ok_or_else(|| "Launcher window not found".to_string())?;

        toggle_launcher(&launcher_window)
            .map_err(|e| format!("Failed to open launcher window: {e}"))?;
        Ok(())
    }

    async fn hide_hover_window<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        let hover_window = app_handle
            .get_window("hover")
            .ok_or_else(|| "Hover window not found".to_string())?;

        hover_window
            .hide()
            .map_err(|e| format!("Failed to hide hover window: {e}"))?;
        Ok(())
    }

    async fn show_hover_window<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        // Get or create hover window
        let hover_window = app_handle.get_window("hover");

        hover_window
            .expect("Hover window not found")
            .show()
            .map_err(|e| format!("Failed to show hover window: {e}"))?;
        Ok(())
    }
}
