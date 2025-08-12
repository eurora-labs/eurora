use tauri::AppHandle;
use tracing::info;
pub(crate) mod state {

    use std::{collections::BTreeMap, sync::Arc};

    use tauri::AppHandle;

    pub(crate) mod event {
        use anyhow::{Context, Result};
        use serde_json;
        use tauri::Emitter;

        /// A change we want to inform the frontend about.
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

    /// State associated to windows
    /// Note that this type is managed in Tauri and thus needs to be `Send` and `Sync`.
    #[derive(Clone)]
    pub struct WindowState {
        _app_handle: AppHandle,
        /// The state for every open application window.
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

        /// Remove the state associated with `window`, typically upon its destruction.
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
    .disable_drag_drop_handler()
    .min_inner_size(800.0, 600.0)
    .inner_size(1160.0, 720.0)
    .build()?;
    Ok(window)
}

#[cfg(not(target_os = "macos"))]
pub fn create_launcher(
    handle: &AppHandle,
    label: &str,
    window_relative_url: String,
) -> tauri::Result<tauri::WebviewWindow> {
    info!("creating window '{label}' created at '{window_relative_url}'");

    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
        // #[cfg(debug_assertions)]
        // tauri::WebviewUrl::External(Url::parse("http://localhost:1420/launcher").unwrap()),
        // #[cfg(not(debug_assertions))]
        // tauri::WebviewUrl::External(Url::parse("http://tauri.localhost/l`auncher").unwrap()),
    )
    .resizable(true)
    .inner_size(1024.0, 500.0)
    .disable_drag_drop_handler()
    .decorations(false)
    .always_on_top(true)
    .center()
    .visible(false)
    .build()?;

    Ok(window)
}

#[cfg(not(target_os = "macos"))]
pub fn create_hover(
    handle: &AppHandle,
    label: &str,
    window_relative_url: String,
) -> tauri::Result<tauri::WebviewWindow> {
    info!("creating window '{label}' created at '{window_relative_url}'");
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
    )
    .inner_size(50.0, 50.0)
    .max_inner_size(50.0, 50.0)
    .resizable(true)
    // .disable_drag_drop_handler()
    .decorations(false)
    .always_on_top(true)
    .transparent(true)
    .shadow(false)
    .skip_taskbar(true)
    // .position(0.0, 0.0)
    // .center()
    .visible(true)
    .build()?;

    Ok(window)
}

#[cfg(target_os = "macos")]
pub fn create(
    handle: &AppHandle,
    label: &str,
    window_relative_url: String,
) -> tauri::Result<tauri::WebviewWindow> {
    info!("creating window '{label}' created at '{window_relative_url}'");
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
#[cfg(target_os = "macos")]
pub fn create_launcher(
    handle: &AppHandle,
    label: &str,
    window_relative_url: String,
) -> tauri::Result<tauri::WebviewWindow> {
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
        // #[cfg(debug_assertions)]
        // tauri::WebviewUrl::External(Url::parse("http://localhost:1420/launcher").unwrap()),
        // #[cfg(not(debug_assertions))]
        // tauri::WebviewUrl::External(Url::parse("http://tauri.localhost/launcher").unwrap()),
    )
    .resizable(false)
    .min_inner_size(575.0, 500.0)
    .inner_size(575.0, 500.0)
    .disable_drag_drop_handler()
    .decorations(false)
    .always_on_top(true)
    .center()
    .title_bar_style(tauri::TitleBarStyle::Overlay)
    .hidden_title(true)
    .visible(false)
    .content_protected(true)
    .build()?;

    Ok(window)
}

#[cfg(target_os = "macos")]
pub fn create_hover(
    handle: &AppHandle,
    label: &str,
    window_relative_url: String,
) -> tauri::Result<tauri::WebviewWindow> {
    info!("creating window '{label}' created at '{window_relative_url}'");
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
    )
    .inner_size(50.0, 50.0)
    .max_inner_size(50.0, 50.0)
    .resizable(false)
    // .disable_drag_drop_handler()
    .decorations(false)
    .title_bar_style(tauri::TitleBarStyle::Transparent)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    // .position(0.0, 0.0)
    // .center()
    .visible(true)
    .build()?;

    Ok(window)
}
