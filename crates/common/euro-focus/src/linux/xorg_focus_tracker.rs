use crate::{FerrousFocusError, FerrousFocusResult, FocusTrackerConfig, FocusedWindow};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

#[cfg(feature = "async")]
use std::future::Future;
use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ConnectionExt, EventMask, PropertyNotifyEvent,
        },
    },
    rust_connection::RustConnection,
};

pub fn track_focus<F>(on_focus: F, config: &FocusTrackerConfig) -> FerrousFocusResult<()>
where
    F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
{
    run(on_focus, None, config)
}

pub fn track_focus_with_stop<F>(
    on_focus: F,
    stop_signal: &AtomicBool,
    config: &FocusTrackerConfig,
) -> FerrousFocusResult<()>
where
    F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
{
    run(on_focus, Some(stop_signal), config)
}

#[cfg(feature = "async")]
pub async fn track_focus_async<F, Fut>(
    on_focus: F,
    config: &FocusTrackerConfig,
) -> FerrousFocusResult<()>
where
    F: FnMut(FocusedWindow) -> Fut,
    Fut: Future<Output = FerrousFocusResult<()>>,
{
    run_async(on_focus, None, config).await
}

#[cfg(feature = "async")]
pub async fn track_focus_async_with_stop<F, Fut>(
    on_focus: F,
    stop_signal: &AtomicBool,
    config: &FocusTrackerConfig,
) -> FerrousFocusResult<()>
where
    F: FnMut(FocusedWindow) -> Fut,
    Fut: Future<Output = FerrousFocusResult<()>>,
{
    run_async(on_focus, Some(stop_signal), config).await
}

#[cfg(feature = "async")]
async fn run_async<F, Fut>(
    mut on_focus: F,
    stop_signal: Option<&AtomicBool>,
    config: &FocusTrackerConfig,
) -> FerrousFocusResult<()>
where
    F: FnMut(FocusedWindow) -> Fut,
    Fut: Future<Output = FerrousFocusResult<()>>,
{
    use std::sync::Arc;
    use tokio::sync::mpsc;

    // Create a channel for communicating focus events from blocking thread to async context
    let (tx, mut rx) = mpsc::unbounded_channel::<FocusedWindow>();

    // Clone config for the blocking task
    let config_clone = config.clone();

    // Create an internal stop signal for the blocking task
    let internal_stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&internal_stop);
    let cleanup_stop = Arc::clone(&internal_stop);

    // Spawn a blocking task for X11 operations (X11 is inherently blocking)
    let blocking_handle = tokio::task::spawn_blocking(move || -> FerrousFocusResult<()> {
        // ── X11 setup ──────────────────────────────────────────────────────────────
        let (conn, screen_num) = connect_to_x11()?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        let atoms = setup_atoms(&conn)?;
        setup_root_window_monitoring(&conn, root)?;

        // Track the currently focused window to monitor its title changes
        let mut current_focused_window: Option<u32> = None;
        // Cache the icon for the currently focused window (only fetch on app change)
        let mut cached_icon: Option<image::RgbaImage> = None;

        // ── Get initial focused window ─────────────────────────────────────────────
        // Fire an immediate event with the currently focused window (like Windows/macOS)
        if let Ok(Some(window)) = get_active_window(&conn, root, atoms.net_active_window) {
            match get_window_info(&conn, window, &atoms) {
                Ok(mut focused_window) => {
                    // Initial window - fetch icon
                    let icon =
                        get_icon_data(&conn, window, atoms.net_wm_icon, &config_clone.icon).ok();
                    cached_icon = icon.clone();
                    focused_window.icon = icon;

                    // Send initial window info to async context via channel
                    if tx.send(focused_window).is_err() {
                        // Channel closed, async task has been dropped
                        info!("Async task dropped before initial event, stopping X11 event loop");
                        return Ok(());
                    }
                    // Set up monitoring for this window
                    current_focused_window = Some(window);
                    let _ = conn.change_window_attributes(
                        window,
                        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
                    );
                    let _ = flush_connection(&conn);
                }
                Err(e) => {
                    info!("Failed to get initial window info: {}", e);
                }
            }
        }

        // ── Event loop ─────────────────────────────────────────────────────────────
        loop {
            // Check stop signal (either internal or external)
            if thread_stop.load(Ordering::Acquire) {
                break;
            }

            // Use non-blocking poll to allow checking stop signal
            let event = match conn.poll_for_event() {
                Ok(Some(e)) => e,
                Ok(None) => {
                    // No event available, sleep briefly
                    // Note: We use std::thread::sleep here because we're in a blocking context
                    std::thread::sleep(config_clone.poll_interval);
                    continue;
                }
                Err(e) => {
                    info!("X11 error: {e}");
                    // Note: We use std::thread::sleep here because we're in a blocking context
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            if let Event::PropertyNotify(PropertyNotifyEvent { atom, window, .. }) = event {
                let mut should_emit_focus_event = false;
                let mut new_window: Option<u32> = None;
                let mut is_focus_change = false;

                // Check if this is an active window change
                if atom == atoms.net_active_window && window == root {
                    // Active window changed
                    match get_active_window(&conn, root, atoms.net_active_window) {
                        Ok(win) => {
                            new_window = win;
                            should_emit_focus_event = true;
                            is_focus_change = true;

                            // Update monitoring for the new focused window
                            update_window_monitoring(
                                &conn,
                                &mut current_focused_window,
                                new_window,
                            );
                        }
                        Err(e) => {
                            info!("Failed to get active window: {}", e);
                            continue;
                        }
                    }
                }
                // Check if this is a title change on the currently focused window
                else if atom == atoms.net_wm_name && Some(window) == current_focused_window {
                    // Title changed on the focused window - don't fetch icon again
                    new_window = current_focused_window;
                    should_emit_focus_event = true;
                    is_focus_change = false;
                }

                if should_emit_focus_event && let Some(window) = new_window {
                    match get_window_info(&conn, window, &atoms) {
                        Ok(mut focused_window) => {
                            // Only fetch icon when the focused app changes, not on title changes
                            if is_focus_change {
                                let icon = get_icon_data(
                                    &conn,
                                    window,
                                    atoms.net_wm_icon,
                                    &config_clone.icon,
                                )
                                .ok();
                                cached_icon = icon.clone();
                                focused_window.icon = icon;
                            } else {
                                focused_window.icon = cached_icon.clone();
                            }

                            // Send to async context via channel
                            if tx.send(focused_window).is_err() {
                                // Channel closed, async task has been dropped
                                info!("Async task dropped, stopping X11 event loop");
                                break;
                            }
                        }
                        Err(e) => {
                            info!("Failed to get window info for window {}: {}", window, e);
                        }
                    }
                }
            }

            if let Err(e) = flush_connection(&conn) {
                info!("Failed to flush connection: {}", e);
            }
        }

        Ok(())
    });

    // Process focus events in async context
    // When this task is cancelled (dropped), cleanup will happen via the guard below
    let result = async {
        loop {
            // Check external stop signal if provided
            if let Some(external_stop) = stop_signal
                && external_stop.load(Ordering::Acquire)
            {
                info!("External stop signal detected");
                break;
            }

            // Use timeout to periodically check the stop signal
            match tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await {
                Ok(Some(focused_window)) => {
                    if let Err(e) = on_focus(focused_window).await {
                        info!("Focus event handler failed: {}", e);
                        // Continue processing instead of propagating the error
                    }
                }
                Ok(None) => {
                    // Channel closed
                    break;
                }
                Err(_) => {
                    // Timeout - continue to check stop signal
                    continue;
                }
            }
        }
        Ok::<(), FerrousFocusError>(())
    }
    .await;

    // Signal the X11 thread to stop (this runs whether we complete normally or get cancelled)
    info!("Async task ending, signaling X11 thread to stop");
    cleanup_stop.store(true, Ordering::Release);

    // Drop the receiver to close the channel, which will also signal the thread
    drop(rx);

    // Give the thread a brief moment to notice the stop signal and exit cleanly
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Wait for blocking task to finish and get result
    match blocking_handle.await {
        Ok(Ok(())) => {
            info!("X11 event loop completed successfully");
            result
        }
        Ok(Err(e)) => {
            info!("X11 event loop error: {}", e);
            Err(e)
        }
        Err(e) => {
            let err_msg = format!("X11 blocking task failed: {}", e);
            info!("{}", err_msg);
            Err(FerrousFocusError::Platform(err_msg))
        }
    }
}

fn run<F>(
    mut on_focus: F,
    stop_signal: Option<&AtomicBool>,
    config: &FocusTrackerConfig,
) -> FerrousFocusResult<()>
where
    F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
{
    // ── X11 setup ──────────────────────────────────────────────────────────────
    let (conn, screen_num) = connect_to_x11()?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let atoms = setup_atoms(&conn)?;
    setup_root_window_monitoring(&conn, root)?;

    // Track the currently focused window to monitor its title changes
    let mut current_focused_window: Option<u32> = None;
    // Cache the icon for the currently focused window (only fetch on app change)
    let mut cached_icon: Option<image::RgbaImage> = None;

    // ── Get initial focused window ─────────────────────────────────────────────
    // Fire an immediate event with the currently focused window (like Windows/macOS)
    if let Ok(Some(window)) = get_active_window(&conn, root, atoms.net_active_window) {
        match get_window_info(&conn, window, &atoms) {
            Ok(mut focused_window) => {
                // Initial window - fetch icon
                let icon = get_icon_data(&conn, window, atoms.net_wm_icon, &config.icon).ok();
                cached_icon = icon.clone();
                focused_window.icon = icon;

                if let Err(e) = on_focus(focused_window) {
                    info!("Initial focus event handler failed: {}", e);
                }
                // Set up monitoring for this window
                current_focused_window = Some(window);
                let _ = conn.change_window_attributes(
                    window,
                    &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
                );
                let _ = flush_connection(&conn);
            }
            Err(e) => {
                info!("Failed to get initial window info: {}", e);
            }
        }
    }

    // ── Event loop ─────────────────────────────────────────────────────────────
    loop {
        // Check stop signal before polling for events
        if should_stop(stop_signal) {
            break;
        }

        let event = get_next_event(&conn, stop_signal, config)?;

        if let Event::PropertyNotify(PropertyNotifyEvent { atom, window, .. }) = event {
            let mut should_emit_focus_event = false;
            let mut new_window: Option<u32> = None;
            let mut is_focus_change = false;

            // Check if this is an active window change
            if atom == atoms.net_active_window && window == root {
                // Active window changed
                match get_active_window(&conn, root, atoms.net_active_window) {
                    Ok(win) => {
                        new_window = win;
                        should_emit_focus_event = true;
                        is_focus_change = true;

                        // Update monitoring for the new focused window
                        update_window_monitoring(&conn, &mut current_focused_window, new_window);
                    }
                    Err(e) => {
                        info!("Failed to get active window: {}", e);
                        continue;
                    }
                }
            }
            // Check if this is a title change on the currently focused window
            else if atom == atoms.net_wm_name && Some(window) == current_focused_window {
                // Title changed on the focused window - don't fetch icon again
                new_window = current_focused_window;
                should_emit_focus_event = true;
                is_focus_change = false;
            }

            if should_emit_focus_event && let Some(window) = new_window {
                match get_window_info(&conn, window, &atoms) {
                    Ok(mut focused_window) => {
                        // Only fetch icon when the focused app changes, not on title changes
                        if is_focus_change {
                            let icon =
                                get_icon_data(&conn, window, atoms.net_wm_icon, &config.icon).ok();
                            cached_icon = icon.clone();
                            focused_window.icon = icon;
                        } else {
                            focused_window.icon = cached_icon.clone();
                        }

                        if let Err(e) = on_focus(focused_window) {
                            info!("Focus event handler failed: {}", e);
                            // Continue processing instead of propagating the error
                        }
                    }
                    Err(e) => {
                        info!("Failed to get window info for window {}: {}", window, e);
                    }
                }
            }
        }

        flush_connection(&conn)?;
    }

    Ok(())
}

/* ------------------------------------------------------------ */
/* Helper structs and functions                                  */
/* ------------------------------------------------------------ */

#[derive(Debug, Clone)]
struct X11Atoms {
    net_active_window: u32,
    net_wm_name: u32,
    net_wm_pid: u32,
    utf8_string: u32,
    net_wm_icon: u32,
}

/// Check if the stop signal is set.
fn should_stop(stop_signal: Option<&AtomicBool>) -> bool {
    stop_signal.is_some_and(|stop| stop.load(Ordering::Acquire))
}

/// Connect to X11 server with proper error handling.
fn connect_to_x11() -> FerrousFocusResult<(RustConnection, usize)> {
    RustConnection::connect(None).map_err(|e| {
        let error_str = e.to_string();
        // Check if this is a "no display" error
        if error_str.contains("DISPLAY")
            || error_str.contains("display")
            || error_str.contains("No such file or directory")
        {
            FerrousFocusError::NoDisplay
        } else {
            FerrousFocusError::Platform(error_str)
        }
    })
}

/// Setup all required X11 atoms.
fn setup_atoms<C: Connection>(conn: &C) -> FerrousFocusResult<X11Atoms> {
    Ok(X11Atoms {
        net_active_window: get_atom(conn, b"_NET_ACTIVE_WINDOW")?,
        net_wm_name: get_atom(conn, b"_NET_WM_NAME")?,
        net_wm_pid: get_atom(conn, b"_NET_WM_PID")?,
        utf8_string: get_atom(conn, b"UTF8_STRING")?,
        net_wm_icon: get_atom(conn, b"_NET_WM_ICON")?,
    })
}

/// Setup monitoring for the root window.
fn setup_root_window_monitoring<C: Connection>(conn: &C, root: u32) -> FerrousFocusResult<()> {
    conn.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    )
    .map_err(|e| FerrousFocusError::Platform(e.to_string()))?;

    conn.flush()
        .map_err(|e| FerrousFocusError::Platform(e.to_string()))?;

    Ok(())
}

/// Get the next X11 event, handling both polling and blocking modes.
fn get_next_event<C: Connection>(
    conn: &C,
    stop_signal: Option<&AtomicBool>,
    config: &FocusTrackerConfig,
) -> FerrousFocusResult<Event> {
    match stop_signal {
        Some(_) => {
            // Use polling when stop signal is available
            loop {
                match conn.poll_for_event() {
                    Ok(Some(e)) => return Ok(e),
                    Ok(None) => {
                        // No event available, sleep briefly to avoid busy waiting
                        std::thread::sleep(config.poll_interval);
                        continue;
                    }
                    Err(e) => {
                        info!("X11 error: {e}");
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    }
                }
            }
        }
        None => {
            // Use blocking wait when no stop signal
            loop {
                match conn.wait_for_event() {
                    Ok(e) => return Ok(e),
                    Err(e) => {
                        info!("X11 error: {e}");
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    }
                }
            }
        }
    }
}

/// Update window monitoring when focus changes.
fn update_window_monitoring<C: Connection>(
    conn: &C,
    current_focused_window: &mut Option<u32>,
    new_window: Option<u32>,
) {
    // Stop monitoring the old window
    if let Some(old_win) = *current_focused_window {
        let _ = conn.change_window_attributes(
            old_win,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::NO_EVENT),
        );
    }

    // Start monitoring the new window for title changes
    if let Some(new_win) = new_window {
        let _ = conn.change_window_attributes(
            new_win,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        );
        *current_focused_window = Some(new_win);
    } else {
        *current_focused_window = None;
    }
}

/// Flush the X11 connection.
fn flush_connection<C: Connection>(conn: &C) -> FerrousFocusResult<()> {
    conn.flush()
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to flush connection: {e}")))
}

/// Get window info (process name, title) without fetching the icon.
/// The icon should be fetched separately using `get_icon_data` only when the focused app changes.
fn get_window_info<C: Connection>(
    conn: &C,
    window: u32,
    atoms: &X11Atoms,
) -> FerrousFocusResult<FocusedWindow> {
    // Handle window property queries with graceful error handling
    let title = get_window_name(conn, window, atoms).unwrap_or_else(|e| {
        info!("Failed to get window title for window {}: {}", window, e);
        "<unknown title>".to_string()
    });

    let (process_id, process_name) = get_process_info(conn, window, atoms.net_wm_pid)
        .map(|(pid, name)| (Some(pid), Some(name)))
        .unwrap_or_else(|e| {
            info!("Failed to get process info for window {}: {}", window, e);
            (None, Some("<unknown>".to_string()))
        });

    Ok(FocusedWindow {
        process_id,
        process_name,
        window_title: Some(title),
        icon: None,
    })
}

/// Get an X11 atom by name.
fn get_atom<C: Connection>(conn: &C, name: &[u8]) -> FerrousFocusResult<u32> {
    let cookie = conn
        .intern_atom(false, name)
        .map_err(|e| FerrousFocusError::Platform(e.to_string()))?;

    let reply = cookie
        .reply()
        .map_err(|e| FerrousFocusError::Platform(e.to_string()))?;

    Ok(reply.atom)
}

/// Get the currently active window.
fn get_active_window<C: Connection>(
    conn: &C,
    root: u32,
    net_active_window: u32,
) -> FerrousFocusResult<Option<u32>> {
    let cookie = conn
        .get_property(false, root, net_active_window, AtomEnum::WINDOW, 0, 1)
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to get active window: {e}")))?;

    let reply = cookie
        .reply()
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to get active window: {e}")))?;

    Ok(reply.value32().and_then(|mut v| v.next()))
}

/// Get the name/title of a window.
fn get_window_name<C: Connection>(
    conn: &C,
    window: u32,
    atoms: &X11Atoms,
) -> FerrousFocusResult<String> {
    // Try UTF‑8 first
    match try_get_property_string(conn, window, atoms.net_wm_name, atoms.utf8_string) {
        Ok(Some(title)) => Ok(title),
        _ => {
            // Fallback to the legacy WM_NAME
            try_get_property_string(
                conn,
                window,
                AtomEnum::WM_NAME.into(),
                AtomEnum::STRING.into(),
            )
            .and_then(|opt| {
                opt.ok_or_else(|| FerrousFocusError::Platform("No window name found".to_string()))
            })
        }
    }
}

/// Helper to get a string property from X11.
fn try_get_property_string<C: Connection>(
    conn: &C,
    window: u32,
    property: u32,
    property_type: u32,
) -> FerrousFocusResult<Option<String>> {
    let cookie = conn
        .get_property(false, window, property, property_type, 0, u32::MAX)
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to get property: {e}")))?;

    let reply = cookie
        .reply()
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to get property: {e}")))?;

    if reply.value_len > 0 {
        Ok(Some(String::from_utf8_lossy(&reply.value).into_owned()))
    } else {
        Ok(None)
    }
}

/// Get both the process ID and process name for a window.
fn get_process_info<C: Connection>(
    conn: &C,
    window: u32,
    net_wm_pid: u32,
) -> FerrousFocusResult<(u32, String)> {
    // fetch the PID stored in _NET_WM_PID
    let cookie = conn
        .get_property(false, window, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to get PID: {e}")))?;

    let reply = cookie
        .reply()
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to get PID: {e}")))?;

    let pid = reply
        .value32()
        .and_then(|mut v| v.next())
        .ok_or_else(|| FerrousFocusError::Platform("No PID found for window".to_string()))?;

    // read /proc/<pid>/comm (single line: executable name)
    let process_name = std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .or_else(|_| {
            std::fs::read_link(format!("/proc/{pid}/exe")).map(|p| p.to_string_lossy().into())
        })
        .map(|name| name.trim_end_matches('\n').to_owned())
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to get process name: {e}")))?;

    Ok((pid, process_name))
}

/// Resize an image to the specified dimensions using the specified filter type
fn resize_icon(
    image: image::RgbaImage,
    target_size: u32,
    filter_type: image::imageops::FilterType,
) -> image::RgbaImage {
    // Only resize if the image is not already the target size
    if image.width() == target_size && image.height() == target_size {
        return image;
    }

    image::imageops::resize(&image, target_size, target_size, filter_type)
}

/// Get icon data for a window and return it as an image::RgbaImage.
fn get_icon_data<C: Connection>(
    conn: &C,
    window: u32,
    net_wm_icon: u32,
    icon_config: &crate::config::IconConfig,
) -> FerrousFocusResult<image::RgbaImage> {
    let cookie = conn
        .get_property(
            false,
            window,
            net_wm_icon,
            AtomEnum::CARDINAL,
            0,
            u32::MAX / 4, // Limit size to avoid huge icons
        )
        .map_err(|e| {
            FerrousFocusError::Platform(format!("Failed to request icon property: {e}"))
        })?;

    let reply = cookie
        .reply()
        .map_err(|e| FerrousFocusError::Platform(format!("Failed to get icon property: {e}")))?;

    if reply.value_len == 0 {
        return Err(FerrousFocusError::Unsupported);
    }

    let values: Vec<u32> = reply
        .value32()
        .ok_or_else(|| {
            FerrousFocusError::Platform("Failed to parse icon data as 32-bit values".to_string())
        })?
        .collect();

    if values.len() < 2 {
        return Err(FerrousFocusError::Platform(
            "Invalid icon data: missing width/height".to_string(),
        ));
    }

    let width = values[0];
    let height = values[1];

    if width == 0 || height == 0 {
        return Err(FerrousFocusError::Platform(
            "Invalid icon dimensions".to_string(),
        ));
    }

    let expected_pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| FerrousFocusError::Platform("Icon dimensions overflow".into()))?;
    let available_pixels = values.len() - 2; // Subtract width and height

    if available_pixels < expected_pixels {
        return Err(FerrousFocusError::Platform(format!(
            "Insufficient pixel data: expected {expected_pixels}, got {available_pixels}",
        )));
    }

    // Convert ARGB u32 values to RGBA u8 bytes
    let mut pixels = Vec::with_capacity(
        expected_pixels
            .checked_mul(4)
            .ok_or_else(|| FerrousFocusError::Platform("Icon dimensions overflow".into()))?,
    );

    for &argb in &values[2..2 + expected_pixels] {
        // Extract ARGB components (native endian)
        let a = ((argb >> 24) & 0xFF) as u8;
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;

        // Store as RGBA
        pixels.push(r);
        pixels.push(g);
        pixels.push(b);
        pixels.push(a);
    }

    // Create RgbaImage from the pixel data
    let mut image = image::RgbaImage::from_raw(width, height, pixels).ok_or_else(|| {
        FerrousFocusError::Platform("Failed to create RgbaImage from pixel data".to_string())
    })?;

    // Resize the icon if needed
    if let Some(target_size) = icon_config.size {
        image = resize_icon(image, target_size, icon_config.filter_type);
    }

    Ok(image)
}
