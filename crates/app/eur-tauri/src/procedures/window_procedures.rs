use tauri::{Manager, Runtime};
#[taurpc::procedures(
    path = "window",
    export_to = "../../../packages/tauri-bindings/src/lib/gen/bindings.ts"
)]
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
        let window = app_handle.get_window("launcher").unwrap();
        let current_size = window.outer_size().unwrap();
        let new_height = height as f64 * scale_factor;
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: current_size.width,
            // height: new_height as u32 + 72,
            height: new_height as u32,
        }));
        Ok(())
    }
}
