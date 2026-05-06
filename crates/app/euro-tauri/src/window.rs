use tauri::{AppHandle, Manager, Runtime};

/// Label of the application's primary window. Used both at creation time and
/// by `show_and_focus_main` to look the window back up.
pub const MAIN_WINDOW_LABEL: &str = "main";

/// Bring the main window to the foreground from any prior state — hidden,
/// minimized, or simply backgrounded behind another application.
///
/// The three window calls are each individually idempotent; together they
/// cover the state matrix:
/// * `show()` reverses an explicit `hide()` (e.g. autostart launch).
/// * `unminimize()` restores the window from the dock/taskbar.
/// * `set_focus()` raises and activates the window. On macOS this also calls
///   `NSApplication::activateIgnoringOtherApps`, which is what actually
///   brings a backgrounded app to the front; without it, `show()` and
///   `unminimize()` only adjust window state and the OS leaves the app
///   behind whatever was previously frontmost.
///
/// On macOS we additionally call `AppHandle::show()` first to reverse an
/// application-level hide (Cmd+H or "Hide Others"), which per-window
/// `show()` cannot reach.
pub fn show_and_focus_main<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    app.show()?;

    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or(tauri::Error::WindowNotFound)?;

    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    Ok(())
}

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

#[cfg(target_os = "linux")]
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
    .transparent(true)
    .disable_drag_drop_handler()
    .min_inner_size(800.0, 600.0)
    .inner_size(1160.0, 720.0)
    .build()?;
    Ok(window)
}

#[cfg(target_os = "windows")]
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
