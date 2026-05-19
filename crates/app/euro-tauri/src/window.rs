use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, Runtime};

/// Label of the application's primary window. Used both at creation time and
/// by `show_and_focus_main` to look the window back up.
pub const MAIN_WINDOW_LABEL: &str = "main";

/// Label of the floating "ask" overlay — the compact input bar the
/// user types into when invoking the ask flow via the global hotkey
/// or tray entry.
pub const ASK_WINDOW_LABEL: &str = "ask-overlay";

/// Label of the answer overlay — the taller pane that opens after a
/// submission and streams the response. Reused across invocations so
/// follow-up questions land in the same surface rather than stacking
/// up windows.
pub const ANSWER_WINDOW_LABEL: &str = "ask-answer";

/// Width, in logical pixels, shared by both overlay windows. The ask
/// bar and the answer pane must align horizontally because the answer
/// pane is anchored directly above or below the bar.
pub const OVERLAY_WIDTH: f64 = 640.0;

/// Logical height of the compact ask bar. Tall enough for a single-row
/// input plus the focused-app icon, short enough to read as a bar
/// rather than a window.
pub const ASK_HEIGHT: f64 = 96.0;

/// Logical height of the answer pane. Matches the spec; large enough
/// to render a meaningful chunk of a streamed response without
/// dominating the screen.
pub const ANSWER_HEIGHT: f64 = 500.0;

/// Gap, in logical pixels, between the ask bar and the answer pane
/// when they are stacked.
pub const OVERLAY_GAP: f64 = 8.0;

/// Vertical inset of the ask bar from the top of the active monitor's
/// work area when no prior position is known. Matches the canonical
/// "spotlight" anchor — high enough to leave the menubar untouched on
/// macOS, low enough to feel deliberate.
pub const ASK_TOP_INSET: f64 = 120.0;

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
    // Linux CSD: we draw our own decorations (rounded corners + shadow in CSS),
    // so the window is decoration-less and transparent. `shadow(true)` is a
    // hint that some compositors (Mutter) honor for borderless windows; the
    // visual fallback comes from the CSS box-shadow on the app shell.
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        label,
        tauri::WebviewUrl::App(window_relative_url.into()),
    )
    .resizable(true)
    .title(handle.package_info().name.clone())
    .decorations(false)
    .transparent(true)
    .shadow(true)
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
    .shadow(true)
    .disable_drag_drop_handler()
    .min_inner_size(800.0, 600.0)
    .inner_size(1160.0, 720.0)
    .build()?;

    // Opt the borderless window back into the Windows 11 system corner
    // rounding and drop shadow that `decorations(false)` otherwise removes.
    // No-op on Windows 10 (the DWM ignores unknown attribute values).
    apply_windows_corner_rounding(&window);

    Ok(window)
}

#[cfg(target_os = "windows")]
pub(crate) fn apply_windows_corner_rounding(window: &tauri::WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{
        DWM_WINDOW_CORNER_PREFERENCE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        DwmSetWindowAttribute,
    };

    let Ok(hwnd) = window.hwnd() else { return };
    let preference: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
    // SAFETY: `hwnd` is a live HWND owned by Tauri for the lifetime of the
    // window, and we hand DWM a pointer to a `DWM_WINDOW_CORNER_PREFERENCE`
    // value of the matching size — exactly what the attribute expects.
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const _,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        )
    };
    if let Err(error) = result {
        tracing::warn!(?error, "DwmSetWindowAttribute(corner_preference) failed");
    }
}

#[cfg(target_os = "macos")]
pub fn create(
    handle: &AppHandle,
    label: &str,
    window_relative_url: String,
) -> tauri::Result<tauri::WebviewWindow> {
    // macOS keeps native window decorations: AppKit rounds the window,
    // draws the system shadow, and manages edge resize. We only ask it to
    // hide the title text and use the overlay style so our custom titlebar
    // can render under the traffic lights.
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
    // The y value here is NOT a position-from-top; wry uses it as the
    // height of the title-bar container *minus the button height*
    // (see wry's `inset_traffic_lights`). The buttons stay anchored at
    // their natural ~7px offset from the container bottom, so values of
    // y < ~14 actually push the button tops above the container's top
    // edge, where AppKit clips them — that's the "tops cut off" symptom
    // on macOS 26 (Tahoe), whose larger Liquid Glass buttons make the
    // problem more visible. y=16 centers the buttons in our 32px bar
    // on Tahoe and looks correct on pre-Tahoe macOS too. Horizontal
    // 12px matches AppKit's standard inset for unified-style windows.
    .traffic_light_position(tauri::LogicalPosition::new(12.0, 16.0))
    .build()?;
    Ok(window)
}

// --- Overlay (ask + answer) window builders -------------------------------
//
// One builder per OS, per window kind, intentionally spelled out instead
// of collapsed behind a macro: the per-platform attribute set already
// diverges (Linux needs `shadow(true)` for compositor hints, Windows
// gets DWM corner rounding, macOS opts out of the overlay titlebar) and
// hiding that behind a macro would only shave a few lines while making
// the platform branches harder to read.

#[cfg(target_os = "linux")]
pub fn create_ask_window(handle: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    tauri::WebviewWindowBuilder::new(
        handle,
        ASK_WINDOW_LABEL,
        tauri::WebviewUrl::App("ask".into()),
    )
    .title("Eurora Ask")
    .inner_size(OVERLAY_WIDTH, ASK_HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .disable_drag_drop_handler()
    .build()
}

/// Build the answer pane with a frontend URL chosen by the caller.
///
/// The relative URL is threaded through so the caller can encode an
/// initial prompt as `?q=…` and have the SvelteKit page read it
/// synchronously on mount. Use `"answer"` for the empty-state pane;
/// use `format!("answer?q={...}")` for the pre-filled variant.
#[cfg(target_os = "linux")]
pub fn create_answer_window(
    handle: &AppHandle,
    relative_url: &str,
) -> tauri::Result<tauri::WebviewWindow> {
    tauri::WebviewWindowBuilder::new(
        handle,
        ANSWER_WINDOW_LABEL,
        tauri::WebviewUrl::App(relative_url.into()),
    )
    .title("Eurora Answer")
    .inner_size(OVERLAY_WIDTH, ANSWER_HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .disable_drag_drop_handler()
    .build()
}

#[cfg(target_os = "windows")]
pub fn create_ask_window(handle: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        ASK_WINDOW_LABEL,
        tauri::WebviewUrl::App("ask".into()),
    )
    .title("Eurora Ask")
    .inner_size(OVERLAY_WIDTH, ASK_HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .disable_drag_drop_handler()
    .build()?;
    apply_windows_corner_rounding(&window);
    Ok(window)
}

#[cfg(target_os = "windows")]
pub fn create_answer_window(
    handle: &AppHandle,
    relative_url: &str,
) -> tauri::Result<tauri::WebviewWindow> {
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        ANSWER_WINDOW_LABEL,
        tauri::WebviewUrl::App(relative_url.into()),
    )
    .title("Eurora Answer")
    .inner_size(OVERLAY_WIDTH, ANSWER_HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .disable_drag_drop_handler()
    .build()?;
    apply_windows_corner_rounding(&window);
    Ok(window)
}

#[cfg(target_os = "macos")]
pub fn create_ask_window(handle: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    // Overlay windows don't get a titlebar, traffic lights, or system
    // shadow. `visible_on_all_workspaces(true)` keeps the bar reachable
    // from whichever Space the user is currently in — without it the
    // hotkey appears to do nothing when invoked from a Space the bar
    // wasn't summoned on.
    tauri::WebviewWindowBuilder::new(
        handle,
        ASK_WINDOW_LABEL,
        tauri::WebviewUrl::App("ask".into()),
    )
    .title("Eurora Ask")
    .inner_size(OVERLAY_WIDTH, ASK_HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible_on_all_workspaces(true)
    .disable_drag_drop_handler()
    .build()
}

#[cfg(target_os = "macos")]
pub fn create_answer_window(
    handle: &AppHandle,
    relative_url: &str,
) -> tauri::Result<tauri::WebviewWindow> {
    tauri::WebviewWindowBuilder::new(
        handle,
        ANSWER_WINDOW_LABEL,
        tauri::WebviewUrl::App(relative_url.into()),
    )
    .title("Eurora Answer")
    .inner_size(OVERLAY_WIDTH, ANSWER_HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible_on_all_workspaces(true)
    .disable_drag_drop_handler()
    .build()
}

/// Position the ask bar in its canonical anchor — top-center of the
/// monitor that currently has focus — and ensure it is visible and
/// focused. Idempotent: safe to call on a freshly-created window or
/// when re-summoning an already-open bar.
///
/// The position is recomputed on every summon rather than persisted,
/// because the active monitor and its dimensions can change between
/// invocations (multi-monitor setups, resolution changes). The user
/// can still drag the bar within a session; the next summon brings it
/// back to anchor.
pub fn anchor_ask_window<R: Runtime>(window: &tauri::WebviewWindow<R>) -> tauri::Result<()> {
    if let Some(monitor) = window.current_monitor()? {
        let scale = monitor.scale_factor();
        let monitor_pos = monitor.position().to_logical::<f64>(scale);
        let monitor_size = monitor.size().to_logical::<f64>(scale);
        let x = monitor_pos.x + (monitor_size.width - OVERLAY_WIDTH).max(0.0) / 2.0;
        let y = monitor_pos.y + ASK_TOP_INSET;
        window.set_position(LogicalPosition::new(x, y))?;
    }
    window.set_size(LogicalSize::new(OVERLAY_WIDTH, ASK_HEIGHT))?;
    window.show()?;
    window.set_focus()?;
    Ok(())
}

/// Side of the ask bar an answer window can be anchored to.
#[derive(Debug, Clone, Copy)]
enum AnswerAnchor {
    Below,
    Above,
}

/// Position the answer window so it shares the ask bar's x-anchor and
/// width, stacked above or below depending on available screen space.
/// Falls back to the canonical top-center anchor when the ask bar is
/// not currently open — that is the path the deep link / App Intent
/// invocation takes, where no bar ever existed.
pub fn anchor_answer_window<R: Runtime>(
    answer: &tauri::WebviewWindow<R>,
    ask: Option<&tauri::WebviewWindow<R>>,
) -> tauri::Result<()> {
    answer.set_size(LogicalSize::new(OVERLAY_WIDTH, ANSWER_HEIGHT))?;

    let placement = ask
        .map(|bar| compute_answer_placement(bar))
        .transpose()?
        .flatten();

    if let Some(position) = placement {
        answer.set_position(position)?;
    } else if let Some(monitor) = answer.current_monitor()? {
        let scale = monitor.scale_factor();
        let monitor_pos = monitor.position().to_logical::<f64>(scale);
        let monitor_size = monitor.size().to_logical::<f64>(scale);
        let x = monitor_pos.x + (monitor_size.width - OVERLAY_WIDTH).max(0.0) / 2.0;
        let y = monitor_pos.y + ASK_TOP_INSET;
        answer.set_position(LogicalPosition::new(x, y))?;
    }

    answer.show()?;
    answer.set_focus()?;
    Ok(())
}

/// Decide where the answer window should sit relative to a live ask
/// bar. Returns the physical position (Tauri's `set_position` accepts
/// either logical or physical; we go physical because the source
/// values from `outer_position`/`current_monitor` are themselves
/// physical and skipping the back-and-forth to logical avoids
/// rounding drift on fractional-DPR displays).
fn compute_answer_placement<R: Runtime>(
    ask: &tauri::WebviewWindow<R>,
) -> tauri::Result<Option<PhysicalPosition<i32>>> {
    let Some(monitor) = ask.current_monitor()? else {
        return Ok(None);
    };
    let ask_pos = ask.outer_position()?;
    let ask_size = ask.outer_size()?;
    let scale = monitor.scale_factor();
    let gap_px = (OVERLAY_GAP * scale).round() as i32;
    let answer_height_px = (ANSWER_HEIGHT * scale).round() as i32;

    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let monitor_bottom = monitor_pos.y + monitor_size.height as i32;
    let ask_bottom = ask_pos.y + ask_size.height as i32;

    let space_below = monitor_bottom - ask_bottom;
    let space_above = ask_pos.y - monitor_pos.y;

    let anchor = if space_below >= answer_height_px + gap_px {
        AnswerAnchor::Below
    } else if space_above >= answer_height_px + gap_px {
        AnswerAnchor::Above
    } else if space_below >= space_above {
        AnswerAnchor::Below
    } else {
        AnswerAnchor::Above
    };

    let y = match anchor {
        AnswerAnchor::Below => ask_bottom + gap_px,
        AnswerAnchor::Above => ask_pos.y - gap_px - answer_height_px,
    };

    Ok(Some(PhysicalPosition::new(ask_pos.x, y)))
}
