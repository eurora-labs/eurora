use tauri::AppHandle;

pub(crate) mod state {
    use std::{collections::BTreeSet, sync::Arc};

    type WindowLabel = String;
    pub(super) type WindowLabelRef = str;

    #[derive(Clone, Default)]
    pub struct WindowState {
        labels: Arc<parking_lot::Mutex<BTreeSet<WindowLabel>>>,
    }

    impl WindowState {
        pub fn remove(&self, window: &WindowLabelRef) {
            self.labels.lock().remove(window);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn create(
    handle: &AppHandle,
    label: &str,
    window_relative_url: String,
) -> tauri::Result<tauri::WebviewWindow> {
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
    )
    .resizable(true)
    .title(handle.package_info().name.clone())
    .decorations(false)
    .disable_drag_drop_handler()
    .min_inner_size(800.0, 600.0)
    .inner_size(1160.0, 720.0)
    .build()?;
    Ok(window)
}

#[cfg(target_os = "macos")]
pub fn create(
    handle: &AppHandle,
    label: &str,
    window_relative_url: String,
) -> tauri::Result<tauri::WebviewWindow> {
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
    )
    .resizable(true)
    .title(handle.package_info().name.clone())
    .min_inner_size(800.0, 600.0)
    .inner_size(1160.0, 720.0)
    .hidden_title(true)
    .disable_drag_drop_handler()
    .title_bar_style(tauri::TitleBarStyle::Overlay)
    .build()?;
    Ok(window)
}
