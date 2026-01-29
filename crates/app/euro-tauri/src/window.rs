use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

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
    let navigation_handler = handle.clone();
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
    )
    .resizable(true)
    .title(handle.package_info().name.clone())
    .disable_drag_drop_handler()
    .on_navigation(move |url| prevent_in_app_navigation(url, &navigation_handler))
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
    let navigation_handler = handle.clone();
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
    )
    .resizable(true)
    .title(handle.package_info().name.clone())
    .min_inner_size(800.0, 600.0)
    .inner_size(1160.0, 720.0)
    .on_navigation(move |url| prevent_in_app_navigation(url, &navigation_handler))
    .hidden_title(true)
    .disable_drag_drop_handler()
    .title_bar_style(tauri::TitleBarStyle::Overlay)
    .build()?;
    Ok(window)
}

fn prevent_in_app_navigation(url: &Url, handle: &AppHandle) -> bool {
    // 1. Internal app URLs
    let is_internal =
        // bundled app (production)
        url.scheme() == "tauri"
        // dev server (SvelteKit/Tauri dev)
        || (tauri::is_dev()
            && url.scheme() == "http"
            && url.host_str() == Some("localhost"));

    if is_internal {
        return true; // let the webview load your app
    }

    // 2. True external links → open in system browser
    let is_http = url.scheme() == "http" || url.scheme() == "https";
    if is_http {
        let _ = handle.opener().open_url(url.to_string(), None::<&str>);
        return false; // cancel navigation inside the Tauri webview
    }

    // 3. Anything else: allow (or tighten if you need)
    true
}
