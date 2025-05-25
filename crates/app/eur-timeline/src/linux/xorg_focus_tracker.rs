use crate::FocusEvent;
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
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

pub fn track_focus<F>(mut on_focus: F) -> anyhow::Result<()>
where
    F: FnMut(FocusEvent) -> anyhow::Result<()>,
{
    // ── X11 setup ──────────────────────────────────────────────────────────────
    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let net_active_window = atom(&conn, b"_NET_ACTIVE_WINDOW")?;
    let net_wm_name = atom(&conn, b"_NET_WM_NAME")?;
    let net_wm_pid = atom(&conn, b"_NET_WM_PID")?;
    let utf8_string = atom(&conn, b"UTF8_STRING")?;
    let net_wm_icon = atom(&conn, b"_NET_WM_ICON")?;

    conn.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    )?;
    conn.flush()?;

    // Track the currently focused window to monitor its title changes
    let mut current_focused_window: Option<u32> = None;

    // ── Event loop ─────────────────────────────────────────────────────────────
    loop {
        let event = match conn.wait_for_event() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("X11 error: {e}");
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
        };

        if let Event::PropertyNotify(PropertyNotifyEvent { atom, window, .. }) = event {
            let mut should_emit_focus_event = false;
            let mut new_window: Option<u32> = None;

            // Check if this is an active window change
            if atom == net_active_window && window == root {
                // Active window changed
                new_window = active_window(&conn, root, net_active_window)?;
                should_emit_focus_event = true;

                // Update monitoring for the new focused window
                if let Some(old_win) = current_focused_window {
                    // Stop monitoring the old window
                    let _ = conn.change_window_attributes(
                        old_win,
                        &ChangeWindowAttributesAux::new().event_mask(EventMask::NO_EVENT),
                    );
                }

                if let Some(new_win) = new_window {
                    // Start monitoring the new window for title changes
                    let _ = conn.change_window_attributes(
                        new_win,
                        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
                    );
                    current_focused_window = Some(new_win);
                } else {
                    current_focused_window = None;
                }
            }
            // Check if this is a title change on the currently focused window
            else if atom == net_wm_name && Some(window) == current_focused_window {
                // Title changed on the focused window
                new_window = current_focused_window;
                should_emit_focus_event = true;
            }

            if should_emit_focus_event {
                // ── Gather window data ────────────────────────────────────────────
                let win = match new_window {
                    Some(w) => w,
                    None => continue,
                };
                let title = window_name(&conn, win, net_wm_name, utf8_string)?;
                let proc =
                    process_name(&conn, win, net_wm_pid).unwrap_or_else(|_| "<unknown>".into());
                let icon = get_icon_data(&conn, win, net_wm_icon)
                    .ok()
                    .and_then(|d| convert_icon_to_base64(&d).ok())
                    .unwrap_or_default();

                // ── Invoke user-supplied handler ──────────────────────────────────
                on_focus(FocusEvent {
                    process: proc,
                    title,
                    icon_base64: icon,
                })?;
            }
        }

        conn.flush()?;
    }
}

/* ------------------------------------------------------------ */
/* Helper functions                                              */
/* ------------------------------------------------------------ */

fn atom<C: Connection>(conn: &C, name: &[u8]) -> Result<u32> {
    Ok(conn.intern_atom(false, name)?.reply()?.atom)
}

fn active_window<C: Connection>(
    conn: &C,
    root: u32,
    net_active_window: u32,
) -> Result<Option<u32>> {
    match conn.get_property(false, root, net_active_window, AtomEnum::WINDOW, 0, 1) {
        Ok(cookie) => match cookie.reply() {
            Ok(reply) => Ok(reply.value32().and_then(|mut v| v.next())),
            Err(err) => Err(err.into()),
        },
        Err(err) => Err(err.into()),
    }
}

fn window_name<C: Connection>(
    conn: &C,
    window: u32,
    net_wm_name: u32,
    utf8_string: u32,
) -> Result<String> {
    // Try UTF‑8 first
    match conn.get_property(false, window, net_wm_name, utf8_string, 0, u32::MAX) {
        Ok(cookie) => {
            match cookie.reply() {
                Ok(reply) => {
                    if reply.value_len > 0 {
                        return Ok(String::from_utf8_lossy(&reply.value).into_owned());
                    }

                    // Fallback to the legacy WM_NAME
                    match conn.get_property(
                        false,
                        window,
                        AtomEnum::WM_NAME,
                        AtomEnum::STRING,
                        0,
                        u32::MAX,
                    ) {
                        Ok(cookie) => match cookie.reply() {
                            Ok(reply) => Ok(String::from_utf8_lossy(&reply.value).into_owned()),
                            Err(err) => Err(err.into()),
                        },
                        Err(err) => Err(err.into()),
                    }
                }
                Err(err) => Err(err.into()),
            }
        }
        Err(err) => Err(err.into()),
    }
}

fn process_name<C: Connection>(conn: &C, window: u32, net_wm_pid: u32) -> Result<String> {
    // fetch the PID stored in _NET_WM_PID
    let pid = match conn.get_property(false, window, net_wm_pid, AtomEnum::CARDINAL, 0, 1) {
        Ok(cookie) => match cookie.reply() {
            Ok(reply) => match reply.value32().and_then(|mut v| v.next()) {
                Some(pid) => pid,
                None => return Err(anyhow::anyhow!("No PID found for window")),
            },
            Err(err) => return Err(err.into()),
        },
        Err(err) => return Err(err.into()),
    };

    // read /proc/<pid>/comm (single line: executable name)
    match std::fs::read_to_string(format!("/proc/{pid}/comm")).or_else(|_| {
        std::fs::read_link(format!("/proc/{pid}/exe")).map(|p| p.to_string_lossy().into())
    }) {
        Ok(name) => Ok(name.trim_end_matches('\n').to_owned()),
        Err(err) => Err(anyhow::anyhow!("Failed to get process name: {}", err)),
    }
}

fn get_icon_data<C: Connection>(conn: &C, window: u32, net_wm_icon: u32) -> Result<Vec<u32>> {
    match conn.get_property(
        false,
        window,
        net_wm_icon,
        AtomEnum::CARDINAL,
        0,
        u32::MAX / 4, // Limit size to avoid huge icons
    ) {
        Ok(cookie) => {
            match cookie.reply() {
                Ok(reply) => {
                    if reply.value_len == 0 {
                        return Err(anyhow::anyhow!("No icon data available"));
                    }

                    // The icon data is an array of 32-bit values
                    match reply.value32() {
                        Some(values) => Ok(values.collect()),
                        None => Err(anyhow::anyhow!("Failed to extract icon data values")),
                    }
                }
                Err(err) => Err(err.into()),
            }
        }
        Err(err) => Err(err.into()),
    }
}

/// Convert ARGB icon data to a base64 encoded PNG image
fn convert_icon_to_base64(icon_data: &[u32]) -> Result<String> {
    if icon_data.len() < 2 {
        return Err(anyhow::anyhow!("Invalid icon data"));
    }

    let width = icon_data[0] as u32;
    let height = icon_data[1] as u32;

    if width == 0 || height == 0 || width > 1024 || height > 1024 {
        return Err(anyhow::anyhow!("Invalid icon dimensions"));
    }

    // Create an image buffer
    let mut img = ImageBuffer::new(width, height);

    // Fill the image with the icon data
    for y in 0..height {
        for x in 0..width {
            let idx = 2 + (y * width + x) as usize;
            if idx < icon_data.len() {
                let argb = icon_data[idx];
                let a = ((argb >> 24) & 0xFF) as u8;
                let r = ((argb >> 16) & 0xFF) as u8;
                let g = ((argb >> 8) & 0xFF) as u8;
                let b = (argb & 0xFF) as u8;
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
    }

    // Encode the image as PNG in memory
    let mut png_data = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png_data);
        img.write_to(&mut cursor, image::ImageFormat::Png)
            .context("Failed to encode image as PNG")?;
    }

    // Encode the PNG data as base64
    let base64_png = general_purpose::STANDARD.encode(&png_data);

    // Add the data URL prefix
    Ok(format!("data:image/png;base64,{}", base64_png))
}
