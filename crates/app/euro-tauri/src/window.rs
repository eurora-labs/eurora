use tauri::AppHandle;

pub(crate) mod state {

    use std::{collections::BTreeMap, sync::Arc};

    use tauri::AppHandle;

    pub(crate) mod event {
        use anyhow::{Context, Result};
        use serde_json;
        use tauri::Emitter;

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct ChangeForFrontend {
            name: String,
            payload: serde_json::Value,
        }

        impl ChangeForFrontend {
            pub fn send(&self, app_handle: &tauri::AppHandle) -> Result<()> {
                app_handle
                    .emit(&self.name, Some(&self.payload))
                    .context("emit event")?;
                tracing::trace!(event_name = self.name);
                Ok(())
            }
        }
    }

    type WindowLabel = String;
    pub(super) type WindowLabelRef = str;

    #[derive(Clone)]
    pub struct WindowState {
        _app_handle: AppHandle,
        state: Arc<parking_lot::Mutex<BTreeMap<WindowLabel, State>>>,
    }

    struct State {
        _window_id: String,
    }

    impl WindowState {
        pub fn new(app_handle: AppHandle) -> Self {
            Self {
                _app_handle: app_handle,
                state: Default::default(),
            }
        }

        pub fn remove(&self, window: &WindowLabelRef) {
            let mut state_by_label = self.state.lock();
            state_by_label.remove(window);
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
