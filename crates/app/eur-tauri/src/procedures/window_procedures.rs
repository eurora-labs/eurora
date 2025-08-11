use crate::launcher::toggle_launcher_window as toggle_launcher;
use tauri::{Manager, Runtime};
use tracing::info;

#[taurpc::procedures(path = "window")]
pub trait WindowApi {
    async fn get_scale_factor<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        height: f64,
    ) -> Result<f64, String>;

    async fn resize_launcher_window<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        height: u32,
        scale_factor: f64,
    ) -> Result<(), String>;

    async fn open_launcher_window<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String>;

    async fn open_main_window<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;
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
        let current_size = window.inner_size().unwrap();
        let scale_factor = (current_size.height as f64) / (height);
        Ok(scale_factor)
    }

    async fn resize_launcher_window<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        height: u32,
        scale_factor: f64,
    ) -> Result<(), String> {
        info!(
            "resize_launcher_window: height: {}, scale_factor: {}",
            height, scale_factor
        );
        let window = app_handle.get_window("launcher").unwrap();
        let new_height = height as f64 * scale_factor;
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: 1024,
            height: new_height as u32,
        }));
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
            .expect("Failed to set window state");
        main_window
            .show()
            .map_err(|e| format!("Failed to open main window: {e}"))?;

        let launcher_window = app_handle
            .get_window("launcher")
            .ok_or_else(|| "Launcher window not found".to_string())?;

        toggle_launcher(&launcher_window)
            .map_err(|e| format!("Failed to open launcher window: {e}"))?;
        Ok(())
    }
}
