use crate::{FocusTrackerConfig, FocusTrackerError, FocusTrackerResult, FocusedWindow};
use focus_tracker_core::IconConfig;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

pub fn track_focus<F>(on_focus: F, config: &FocusTrackerConfig) -> FocusTrackerResult<()>
where
    F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
{
    run(on_focus, None, config)
}

pub fn track_focus_with_stop<F>(
    on_focus: F,
    stop_signal: &AtomicBool,
    config: &FocusTrackerConfig,
) -> FocusTrackerResult<()>
where
    F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
{
    run(on_focus, Some(stop_signal), config)
}

#[cfg(feature = "async")]
pub async fn track_focus_async<F, Fut>(
    on_focus: F,
    config: &FocusTrackerConfig,
) -> FocusTrackerResult<()>
where
    F: FnMut(FocusedWindow) -> Fut,
    Fut: Future<Output = FocusTrackerResult<()>>,
{
    run_async(on_focus, None, config).await
}

#[cfg(feature = "async")]
pub async fn track_focus_async_with_stop<F, Fut>(
    on_focus: F,
    stop_signal: &AtomicBool,
    config: &FocusTrackerConfig,
) -> FocusTrackerResult<()>
where
    F: FnMut(FocusedWindow) -> Fut,
    Fut: Future<Output = FocusTrackerResult<()>>,
{
    run_async(on_focus, Some(stop_signal), config).await
}

#[cfg(feature = "async")]
async fn run_async<F, Fut>(
    mut on_focus: F,
    stop_signal: Option<&AtomicBool>,
    config: &FocusTrackerConfig,
) -> FocusTrackerResult<()>
where
    F: FnMut(FocusedWindow) -> Fut,
    Fut: Future<Output = FocusTrackerResult<()>>,
{
    use std::sync::Arc;
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::unbounded_channel::<FocusedWindow>();
    let config_clone = config.clone();

    let internal_stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&internal_stop);
    let cleanup_stop = Arc::clone(&internal_stop);

    let blocking_handle = tokio::task::spawn_blocking(move || -> FocusTrackerResult<()> {
        let (conn, screen_num) = connect_to_x11()?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        let atoms = setup_atoms(&conn)?;
        setup_root_window_monitoring(&conn, root)?;

        let mut current_focused_window: Option<u32> = None;
        let mut cached_icon: Option<Arc<image::RgbaImage>> = None;
        let mut consecutive_errors: u32 = 0;

        if let Ok(Some(window)) = get_active_window(&conn, root, atoms.net_active_window) {
            match get_window_info(&conn, window, &atoms) {
                Ok(mut focused_window) => {
                    let icon = get_icon_data(&conn, window, atoms.net_wm_icon, &config_clone.icon)
                        .ok()
                        .map(Arc::new);
                    cached_icon = icon.clone();
                    focused_window.icon = icon;

                    current_focused_window = Some(window);
                    if let Err(e) = conn.change_window_attributes(
                        window,
                        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
                    ) {
                        tracing::info!("Failed to monitor initial window {window}: {e}");
                    }
                    if let Err(e) = flush_connection(&conn) {
                        tracing::info!("Failed to flush after initial monitoring: {e}");
                    }

                    if tx.send(focused_window).is_err() {
                        tracing::info!(
                            "Async task dropped before initial event, stopping X11 event loop"
                        );
                        return Ok(());
                    }
                }
                Err(e) => {
                    tracing::info!("Failed to get initial window info: {}", e);
                }
            }
        }

        loop {
            if thread_stop.load(Ordering::Acquire) {
                break;
            }

            let event = match conn.poll_for_event() {
                Ok(Some(e)) => e,
                Ok(None) => {
                    std::thread::sleep(config_clone.poll_interval);
                    continue;
                }
                Err(e) => {
                    consecutive_errors += 1;
                    tracing::info!(
                        "X11 error ({consecutive_errors}/{MAX_CONSECUTIVE_X11_ERRORS}): {e}"
                    );
                    if consecutive_errors >= MAX_CONSECUTIVE_X11_ERRORS {
                        return Err(FocusTrackerError::platform_with_source(
                            "X11 connection failed repeatedly",
                            e,
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };
            consecutive_errors = 0;

            let Event::PropertyNotify(PropertyNotifyEvent { atom, window, .. }) = event else {
                continue;
            };

            let mut should_emit_focus_event = false;
            let mut new_window: Option<u32> = None;
            let mut is_focus_change = false;

            if atom == atoms.net_active_window && window == root {
                match get_active_window(&conn, root, atoms.net_active_window) {
                    Ok(win) => {
                        new_window = win;
                        should_emit_focus_event = true;
                        is_focus_change = true;

                        update_window_monitoring(&conn, &mut current_focused_window, new_window);
                        if let Err(e) = flush_connection(&conn) {
                            tracing::info!("Failed to flush connection: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::info!("Failed to get active window: {}", e);
                        continue;
                    }
                }
            } else if (atom == atoms.net_wm_name || atom == atoms.wm_name)
                && Some(window) == current_focused_window
            {
                new_window = current_focused_window;
                should_emit_focus_event = true;
                is_focus_change = false;
            }

            if should_emit_focus_event && let Some(window) = new_window {
                match get_window_info(&conn, window, &atoms) {
                    Ok(mut focused_window) => {
                        if is_focus_change {
                            let icon =
                                get_icon_data(&conn, window, atoms.net_wm_icon, &config_clone.icon)
                                    .ok()
                                    .map(Arc::new);
                            cached_icon = icon.clone();
                            focused_window.icon = icon;
                        } else {
                            focused_window.icon = cached_icon.clone();
                        }

                        if tx.send(focused_window).is_err() {
                            tracing::info!("Async task dropped, stopping X11 event loop");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::info!("Failed to get window info for window {}: {}", window, e);
                        if is_focus_change {
                            current_focused_window = None;
                        }
                    }
                }
            }
        }

        Ok(())
    });

    let result = async {
        loop {
            if let Some(external_stop) = stop_signal
                && external_stop.load(Ordering::Acquire)
            {
                tracing::info!("External stop signal detected");
                break;
            }

            match tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await {
                Ok(Some(focused_window)) => {
                    if let Err(e) = on_focus(focused_window).await {
                        tracing::info!("Focus event handler failed: {}", e);
                    }
                }
                Ok(None) => {
                    break;
                }
                Err(_) => {
                    continue;
                }
            }
        }
        Ok::<(), FocusTrackerError>(())
    }
    .await;

    tracing::info!("Async task ending, signaling X11 thread to stop");
    cleanup_stop.store(true, Ordering::Release);

    drop(rx);

    let shutdown_timeout = std::time::Duration::from_secs(3);

    match tokio::time::timeout(shutdown_timeout, blocking_handle).await {
        Ok(Ok(Ok(()))) => {
            tracing::info!("X11 event loop completed successfully");
            result
        }
        Ok(Ok(Err(e))) => {
            tracing::info!("X11 event loop error: {}", e);
            Err(e)
        }
        Ok(Err(e)) => {
            let err_msg = format!("X11 blocking task panicked: {e}");
            tracing::info!("{}", err_msg);
            Err(FocusTrackerError::platform(err_msg))
        }
        Err(_) => {
            tracing::info!("X11 blocking task did not stop within {shutdown_timeout:?}, aborting");
            Err(FocusTrackerError::platform(
                "X11 event loop shutdown timed out",
            ))
        }
    }
}

fn run<F>(
    mut on_focus: F,
    stop_signal: Option<&AtomicBool>,
    config: &FocusTrackerConfig,
) -> FocusTrackerResult<()>
where
    F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
{
    let (conn, screen_num) = connect_to_x11()?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let atoms = setup_atoms(&conn)?;
    setup_root_window_monitoring(&conn, root)?;

    let mut current_focused_window: Option<u32> = None;
    let mut cached_icon: Option<Arc<image::RgbaImage>> = None;

    if let Ok(Some(window)) = get_active_window(&conn, root, atoms.net_active_window) {
        match get_window_info(&conn, window, &atoms) {
            Ok(mut focused_window) => {
                let icon = get_icon_data(&conn, window, atoms.net_wm_icon, &config.icon)
                    .ok()
                    .map(Arc::new);
                cached_icon = icon.clone();
                focused_window.icon = icon;

                current_focused_window = Some(window);
                if let Err(e) = conn.change_window_attributes(
                    window,
                    &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
                ) {
                    tracing::info!("Failed to monitor initial window {window}: {e}");
                }
                if let Err(e) = flush_connection(&conn) {
                    tracing::info!("Failed to flush after initial monitoring: {e}");
                }

                on_focus(focused_window)?;
            }
            Err(e) => {
                tracing::info!("Failed to get initial window info: {}", e);
            }
        }
    }

    loop {
        let event = match get_next_event(&conn, stop_signal, config)? {
            Some(event) => event,
            None => break,
        };

        let Event::PropertyNotify(PropertyNotifyEvent { atom, window, .. }) = event else {
            continue;
        };

        let mut should_emit_focus_event = false;
        let mut new_window: Option<u32> = None;
        let mut is_focus_change = false;

        if atom == atoms.net_active_window && window == root {
            match get_active_window(&conn, root, atoms.net_active_window) {
                Ok(win) => {
                    new_window = win;
                    should_emit_focus_event = true;
                    is_focus_change = true;

                    update_window_monitoring(&conn, &mut current_focused_window, new_window);
                    flush_connection(&conn)?;
                }
                Err(e) => {
                    tracing::info!("Failed to get active window: {}", e);
                    continue;
                }
            }
        } else if (atom == atoms.net_wm_name || atom == atoms.wm_name)
            && Some(window) == current_focused_window
        {
            new_window = current_focused_window;
            should_emit_focus_event = true;
            is_focus_change = false;
        }

        if should_emit_focus_event && let Some(window) = new_window {
            match get_window_info(&conn, window, &atoms) {
                Ok(mut focused_window) => {
                    if is_focus_change {
                        let icon = get_icon_data(&conn, window, atoms.net_wm_icon, &config.icon)
                            .ok()
                            .map(Arc::new);
                        cached_icon = icon.clone();
                        focused_window.icon = icon;
                    } else {
                        focused_window.icon = cached_icon.clone();
                    }

                    on_focus(focused_window)?;
                }
                Err(e) => {
                    tracing::info!("Failed to get window info for window {}: {}", window, e);
                    if is_focus_change {
                        current_focused_window = None;
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct X11Atoms {
    net_active_window: u32,
    net_wm_name: u32,
    wm_name: u32,
    net_wm_pid: u32,
    utf8_string: u32,
    net_wm_icon: u32,
}

fn connect_to_x11() -> FocusTrackerResult<(RustConnection, usize)> {
    RustConnection::connect(None).map_err(|e| {
        let error_str = e.to_string();
        if error_str.contains("DISPLAY")
            || error_str.contains("display")
            || error_str.contains("No such file or directory")
        {
            FocusTrackerError::NoDisplay
        } else {
            FocusTrackerError::platform_with_source("failed to connect to X11", e)
        }
    })
}

fn setup_atoms<C: Connection>(conn: &C) -> FocusTrackerResult<X11Atoms> {
    Ok(X11Atoms {
        net_active_window: get_atom(conn, b"_NET_ACTIVE_WINDOW")?,
        net_wm_name: get_atom(conn, b"_NET_WM_NAME")?,
        wm_name: AtomEnum::WM_NAME.into(),
        net_wm_pid: get_atom(conn, b"_NET_WM_PID")?,
        utf8_string: get_atom(conn, b"UTF8_STRING")?,
        net_wm_icon: get_atom(conn, b"_NET_WM_ICON")?,
    })
}

fn setup_root_window_monitoring<C: Connection>(conn: &C, root: u32) -> FocusTrackerResult<()> {
    conn.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    )
    .map_err(|e| FocusTrackerError::platform_with_source("failed to monitor root window", e))?;

    conn.flush().map_err(|e| {
        FocusTrackerError::platform_with_source("failed to flush after root window monitoring", e)
    })?;

    Ok(())
}

const MAX_CONSECUTIVE_X11_ERRORS: u32 = 10;

/// Returns `Ok(None)` when the stop signal fires, `Ok(Some(event))` on an
/// event, or `Err` on unrecoverable failure.
fn get_next_event<C: Connection>(
    conn: &C,
    stop_signal: Option<&AtomicBool>,
    config: &FocusTrackerConfig,
) -> FocusTrackerResult<Option<Event>> {
    let mut consecutive_errors: u32 = 0;

    match stop_signal {
        Some(stop) => loop {
            if stop.load(Ordering::Acquire) {
                return Ok(None);
            }
            match conn.poll_for_event() {
                Ok(Some(e)) => return Ok(Some(e)),
                Ok(None) => {
                    std::thread::sleep(config.poll_interval);
                }
                Err(e) => {
                    consecutive_errors += 1;
                    tracing::info!(
                        "X11 error ({consecutive_errors}/{MAX_CONSECUTIVE_X11_ERRORS}): {e}"
                    );
                    if consecutive_errors >= MAX_CONSECUTIVE_X11_ERRORS {
                        return Err(FocusTrackerError::platform_with_source(
                            "X11 connection failed repeatedly",
                            e,
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        },
        None => loop {
            match conn.wait_for_event() {
                Ok(e) => return Ok(Some(e)),
                Err(e) => {
                    consecutive_errors += 1;
                    tracing::info!(
                        "X11 error ({consecutive_errors}/{MAX_CONSECUTIVE_X11_ERRORS}): {e}"
                    );
                    if consecutive_errors >= MAX_CONSECUTIVE_X11_ERRORS {
                        return Err(FocusTrackerError::platform_with_source(
                            "X11 connection failed repeatedly",
                            e,
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        },
    }
}

fn update_window_monitoring<C: Connection>(
    conn: &C,
    current_focused_window: &mut Option<u32>,
    new_window: Option<u32>,
) {
    if let Some(old_win) = *current_focused_window
        && let Err(e) = conn.change_window_attributes(
            old_win,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::NO_EVENT),
        )
    {
        tracing::info!("Failed to remove monitoring from window {old_win}: {e}");
    }

    if let Some(new_win) = new_window {
        if let Err(e) = conn.change_window_attributes(
            new_win,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        ) {
            tracing::info!("Failed to add monitoring to window {new_win}: {e}");
        }
        *current_focused_window = Some(new_win);
    } else {
        *current_focused_window = None;
    }
}

fn flush_connection<C: Connection>(conn: &C) -> FocusTrackerResult<()> {
    conn.flush()
        .map_err(|e| FocusTrackerError::platform_with_source("failed to flush X11 connection", e))
}

fn get_window_info<C: Connection>(
    conn: &C,
    window: u32,
    atoms: &X11Atoms,
) -> FocusTrackerResult<FocusedWindow> {
    let title = get_window_name(conn, window, atoms).unwrap_or_else(|e| {
        tracing::info!("Failed to get window title for window {}: {}", window, e);
        "<unknown title>".to_string()
    });

    let (process_id, process_name) = get_process_info(conn, window, atoms.net_wm_pid)?;

    Ok(FocusedWindow {
        process_id,
        process_name,
        window_title: Some(title),
        icon: None,
    })
}

fn get_atom<C: Connection>(conn: &C, name: &[u8]) -> FocusTrackerResult<u32> {
    let cookie = conn
        .intern_atom(false, name)
        .map_err(|e| FocusTrackerError::platform_with_source("failed to intern atom", e))?;

    let reply = cookie
        .reply()
        .map_err(|e| FocusTrackerError::platform_with_source("failed to get atom reply", e))?;

    Ok(reply.atom)
}

fn get_active_window<C: Connection>(
    conn: &C,
    root: u32,
    net_active_window: u32,
) -> FocusTrackerResult<Option<u32>> {
    let cookie = conn
        .get_property(false, root, net_active_window, AtomEnum::WINDOW, 0, 1)
        .map_err(|e| {
            FocusTrackerError::platform_with_source("failed to get active window property", e)
        })?;

    let reply = cookie.reply().map_err(|e| {
        FocusTrackerError::platform_with_source("failed to get active window reply", e)
    })?;

    Ok(reply
        .value32()
        .and_then(|mut v| v.next())
        .filter(|&id| id != 0))
}

fn get_window_name<C: Connection>(
    conn: &C,
    window: u32,
    atoms: &X11Atoms,
) -> FocusTrackerResult<String> {
    match try_get_property_string(conn, window, atoms.net_wm_name, atoms.utf8_string) {
        Ok(Some(title)) => Ok(title),
        _ => try_get_property_string(
            conn,
            window,
            AtomEnum::WM_NAME.into(),
            AtomEnum::STRING.into(),
        )
        .and_then(|opt| opt.ok_or_else(|| FocusTrackerError::platform("no window name found"))),
    }
}

const MAX_STRING_PROPERTY_LEN: u32 = 4096;

fn try_get_property_string<C: Connection>(
    conn: &C,
    window: u32,
    property: u32,
    property_type: u32,
) -> FocusTrackerResult<Option<String>> {
    let cookie = conn
        .get_property(
            false,
            window,
            property,
            property_type,
            0,
            MAX_STRING_PROPERTY_LEN,
        )
        .map_err(|e| FocusTrackerError::platform_with_source("failed to get window property", e))?;

    let reply = cookie.reply().map_err(|e| {
        FocusTrackerError::platform_with_source("failed to get window property reply", e)
    })?;

    if reply.value_len > 0 {
        Ok(Some(String::from_utf8_lossy(&reply.value).into_owned()))
    } else {
        Ok(None)
    }
}

fn get_process_info<C: Connection>(
    conn: &C,
    window: u32,
    net_wm_pid: u32,
) -> FocusTrackerResult<(u32, String)> {
    let cookie = conn
        .get_property(false, window, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
        .map_err(|e| {
            FocusTrackerError::platform_with_source("failed to get window PID property", e)
        })?;

    let reply = cookie.reply().map_err(|e| {
        FocusTrackerError::platform_with_source("failed to get window PID reply", e)
    })?;

    let pid = reply
        .value32()
        .and_then(|mut v| v.next())
        .ok_or_else(|| FocusTrackerError::platform("no PID found for window"))?;

    let process_name = std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .or_else(|_| {
            std::fs::read_link(format!("/proc/{pid}/exe")).map(|p| p.to_string_lossy().into())
        })
        .map(|name| name.trim_end_matches('\n').to_owned())
        .map_err(|e| {
            FocusTrackerError::platform_with_source(
                format!("failed to get process name for pid {pid}"),
                e,
            )
        })?;

    Ok((pid, process_name))
}

fn resize_icon(
    image: image::RgbaImage,
    target_size: u32,
    filter_type: image::imageops::FilterType,
) -> image::RgbaImage {
    if image.width() == target_size && image.height() == target_size {
        return image;
    }

    image::imageops::resize(&image, target_size, target_size, filter_type)
}

struct IconEntry {
    width: u32,
    height: u32,
    pixel_offset: usize,
}

fn parse_icon_entries(values: &[u32]) -> Vec<IconEntry> {
    const MAX_ICON_DIMENSION: u32 = 1024;
    let mut entries = Vec::new();
    let mut offset = 0;

    while offset + 2 <= values.len() {
        let width = values[offset];
        let height = values[offset + 1];
        let pixel_offset = offset + 2;

        if width == 0 || height == 0 || width > MAX_ICON_DIMENSION || height > MAX_ICON_DIMENSION {
            break;
        }

        let pixel_count = match (width as usize).checked_mul(height as usize) {
            Some(n) => n,
            None => break,
        };

        if pixel_offset + pixel_count > values.len() {
            break;
        }

        entries.push(IconEntry {
            width,
            height,
            pixel_offset,
        });

        offset = pixel_offset + pixel_count;
    }

    entries
}

/// Select the best icon for the target size. Preference order:
/// 1. Exact match (returned immediately)
/// 2. Smallest icon larger than the target
/// 3. Largest icon overall
fn select_closest_size_icon(entries: &[IconEntry], target: u32) -> Option<&IconEntry> {
    let mut best_larger: Option<&IconEntry> = None;
    let mut largest: Option<&IconEntry> = None;

    for entry in entries {
        let max_dim = entry.width.max(entry.height);

        if entry.width == target && entry.height == target {
            return Some(entry);
        }

        if entry.width > target
            && entry.height > target
            && best_larger.is_none_or(|b| max_dim < b.width.max(b.height))
        {
            best_larger = Some(entry);
        }

        if largest.is_none_or(|b| max_dim > b.width.max(b.height)) {
            largest = Some(entry);
        }
    }

    best_larger.or(largest)
}

fn decode_icon_entry(values: &[u32], entry: &IconEntry) -> FocusTrackerResult<image::RgbaImage> {
    let pixel_count = (entry.width as usize)
        .checked_mul(entry.height as usize)
        .ok_or_else(|| FocusTrackerError::platform("icon dimensions overflow"))?;

    let mut pixels = Vec::with_capacity(
        pixel_count
            .checked_mul(4)
            .ok_or_else(|| FocusTrackerError::platform("icon buffer size overflow"))?,
    );

    for &argb in &values[entry.pixel_offset..entry.pixel_offset + pixel_count] {
        let a = ((argb >> 24) & 0xFF) as u8;
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;

        pixels.push(r);
        pixels.push(g);
        pixels.push(b);
        pixels.push(a);
    }

    image::RgbaImage::from_raw(entry.width, entry.height, pixels)
        .ok_or_else(|| FocusTrackerError::platform("failed to create RgbaImage from pixel data"))
}

fn get_icon_data<C: Connection>(
    conn: &C,
    window: u32,
    net_wm_icon: u32,
    icon_config: &IconConfig,
) -> FocusTrackerResult<image::RgbaImage> {
    match get_icon_from_x11_property(conn, window, net_wm_icon, icon_config) {
        Ok(image) => return Ok(image),
        Err(e) => {
            if !matches!(e, FocusTrackerError::Unsupported) {
                tracing::info!("_NET_WM_ICON failed: {e}, trying desktop file fallback");
            }
        }
    }

    let (wm_instance, wm_class) =
        get_wm_class(conn, window).ok_or(FocusTrackerError::Unsupported)?;

    let icon_value =
        find_desktop_icon(&wm_instance, &wm_class).ok_or(FocusTrackerError::Unsupported)?;

    let icon_path = resolve_icon_path(&icon_value).ok_or(FocusTrackerError::Unsupported)?;

    tracing::info!(
        "Loading icon from desktop file fallback: {}",
        icon_path.display()
    );
    load_icon_from_file(&icon_path, icon_config)
}

fn get_icon_from_x11_property<C: Connection>(
    conn: &C,
    window: u32,
    net_wm_icon: u32,
    icon_config: &IconConfig,
) -> FocusTrackerResult<image::RgbaImage> {
    let cookie = conn
        .get_property(
            false,
            window,
            net_wm_icon,
            AtomEnum::CARDINAL,
            0,
            u32::MAX / 4,
        )
        .map_err(|e| {
            FocusTrackerError::platform_with_source("failed to request icon property", e)
        })?;

    let reply = cookie.reply().map_err(|e| {
        FocusTrackerError::platform_with_source("failed to get icon property reply", e)
    })?;

    if reply.value_len == 0 {
        return Err(FocusTrackerError::Unsupported);
    }

    let values: Vec<u32> = reply
        .value32()
        .ok_or_else(|| FocusTrackerError::platform("failed to parse icon data as 32-bit values"))?
        .collect();

    let entries = parse_icon_entries(&values);
    if entries.is_empty() {
        return Err(FocusTrackerError::platform(
            "no valid icon entries in _NET_WM_ICON data",
        ));
    }

    let target = icon_config.get_size_or_default();
    let best = select_closest_size_icon(&entries, target)
        .ok_or_else(|| FocusTrackerError::platform("no suitable icon found"))?;

    let mut image = decode_icon_entry(&values, best)?;

    if let Some(target_size) = icon_config.size {
        image = resize_icon(image, target_size, icon_config.filter_type);
    }

    Ok(image)
}

fn get_wm_class<C: Connection>(conn: &C, window: u32) -> Option<(String, String)> {
    let cookie = conn
        .get_property(
            false,
            window,
            u32::from(AtomEnum::WM_CLASS),
            u32::from(AtomEnum::STRING),
            0,
            MAX_STRING_PROPERTY_LEN,
        )
        .ok()?;

    let reply = cookie.reply().ok()?;
    if reply.value_len == 0 {
        return None;
    }

    let raw = &reply.value;
    let parts: Vec<&str> = std::str::from_utf8(raw)
        .ok()?
        .trim_end_matches('\0')
        .splitn(2, '\0')
        .collect();

    if parts.len() == 2 {
        Some((parts[0].to_owned(), parts[1].to_owned()))
    } else if parts.len() == 1 && !parts[0].is_empty() {
        Some((parts[0].to_owned(), parts[0].to_owned()))
    } else {
        None
    }
}

fn xdg_application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    for dir in data_dirs.split(':') {
        if !dir.is_empty() {
            dirs.push(PathBuf::from(dir).join("applications"));
        }
    }

    dirs
}

struct DesktopEntry {
    icon: Option<String>,
    startup_wm_class: Option<String>,
}

fn parse_desktop_file(path: &Path) -> Option<DesktopEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut icon = None;
    let mut startup_wm_class = None;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            if !in_desktop_entry && icon.is_some() {
                break;
            }
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some(value) = line.strip_prefix("Icon=") {
            icon = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("StartupWMClass=") {
            startup_wm_class = Some(value.trim().to_owned());
        }

        if icon.is_some() && startup_wm_class.is_some() {
            break;
        }
    }

    Some(DesktopEntry {
        icon,
        startup_wm_class,
    })
}

fn find_desktop_icon(wm_instance: &str, wm_class: &str) -> Option<String> {
    let dirs = xdg_application_dirs();

    let mut desktop_files: Vec<PathBuf> = Vec::new();
    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "desktop") {
                    desktop_files.push(path);
                }
            }
        }
    }

    for path in &desktop_files {
        if let Some(entry) = parse_desktop_file(path)
            && let Some(ref swc) = entry.startup_wm_class
            && (swc.eq_ignore_ascii_case(wm_instance) || swc.eq_ignore_ascii_case(wm_class))
            && let Some(icon) = entry.icon
        {
            return Some(icon);
        }
    }

    for path in &desktop_files {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && (stem.eq_ignore_ascii_case(wm_instance) || stem.eq_ignore_ascii_case(wm_class))
            && let Some(entry) = parse_desktop_file(path)
            && let Some(icon) = entry.icon
        {
            return Some(icon);
        }
    }

    None
}

fn xdg_icon_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("icons"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/icons"));
    }

    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    for dir in data_dirs.split(':') {
        if !dir.is_empty() {
            dirs.push(PathBuf::from(dir).join("icons"));
        }
    }

    dirs
}

fn resolve_icon_path(icon: &str) -> Option<PathBuf> {
    let path = Path::new(icon);

    if path.is_absolute() {
        return path.exists().then(|| path.to_owned());
    }

    let sizes = [
        "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32",
    ];
    let extensions = ["png", "xpm"];

    for base_dir in xdg_icon_dirs() {
        for size in &sizes {
            for ext in &extensions {
                let candidate = base_dir
                    .join("hicolor")
                    .join(size)
                    .join("apps")
                    .join(format!("{icon}.{ext}"));
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    for ext in &extensions {
        let candidate = PathBuf::from(format!("/usr/share/pixmaps/{icon}.{ext}"));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn load_icon_from_file(
    path: &Path,
    icon_config: &IconConfig,
) -> FocusTrackerResult<image::RgbaImage> {
    let img = image::open(path)
        .map_err(|e| {
            FocusTrackerError::platform_with_source(
                format!("failed to load icon from {}", path.display()),
                e,
            )
        })?
        .into_rgba8();

    let mut image = img;
    if let Some(target_size) = icon_config.size {
        image = resize_icon(image, target_size, icon_config.filter_type);
    }

    Ok(image)
}
