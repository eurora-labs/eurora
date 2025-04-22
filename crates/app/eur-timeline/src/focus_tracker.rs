use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use std::thread;

use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ConnectionExt, EventMask,
            PropertyNotifyEvent,
        },
    },
    rust_connection::RustConnection,
};

use eur_activity::select_strategy_for_process;

pub fn spawn(timeline: &super::Timeline) {
    // Clone the reference to the timeline for the thread
    let timeline = timeline.clone_ref();

    // Run in its own thread so the Tauri event‑loop stays free.
    thread::spawn(move || {
        if let Err(e) = track_focus(timeline) {
            eprintln!("Focus‑tracker exited: {e}");
        }
    });
}

/// Check if an error is an X11 window error (typically for a window that no longer exists)
fn is_window_error(err: &anyhow::Error) -> bool {
    // Check the error message for window-related terms
    let error_msg = err.to_string().to_lowercase();
    error_msg.contains("window") || error_msg.contains("bad drawable")
}

fn track_focus(timeline: super::TimelineRef) -> Result<()> {
    // Connect to the X server
    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root_window_id = screen.root;

    // Atoms we'll need
    let net_active_window = atom(&conn, b"_NET_ACTIVE_WINDOW")?;
    let net_wm_name = atom(&conn, b"_NET_WM_NAME")?;
    let net_wm_pid = atom(&conn, b"_NET_WM_PID")?;
    let utf8_string = atom(&conn, b"UTF8_STRING")?;
    let net_wm_icon = atom(&conn, b"_NET_WM_ICON")?;

    // Ask X to send us property‑change events on the root window
    conn.change_window_attributes(
        root_window_id,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    )?;
    conn.flush()?;

    loop {
        // Handle errors at the event level to prevent the entire tracker from exiting
        let event = match conn.wait_for_event() {
            Ok(event) => event,
            Err(err) => {
                eprintln!("Error waiting for X11 event: {}", err);
                // Short delay to avoid tight loop if there's a persistent error
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
        };

        if let Event::PropertyNotify(PropertyNotifyEvent { atom, .. }) = event {
            // Has the active window changed?
            if atom == net_active_window {
                // Get the active window, handling potential errors
                let win = match active_window(&conn, root_window_id, net_active_window) {
                    Ok(Some(win)) => win,
                    Ok(None) => continue, // No active window
                    Err(err) => {
                        if is_window_error(&err) {
                            // Window error is expected when windows are closed
                            eprintln!(
                                "Window error when getting active window (likely closed): {}",
                                err
                            );
                        } else {
                            eprintln!("Error getting active window: {}", err);
                        }
                        continue;
                    }
                };

                // Get window title, handling potential errors
                let title = match window_name(&conn, win, net_wm_name, utf8_string) {
                    Ok(title) => title,
                    Err(err) => {
                        if is_window_error(&err) {
                            eprintln!("Window error when getting window name (likely closed)");
                        } else {
                            eprintln!("Error getting window name: {}", err);
                        }
                        continue;
                    }
                };

                // Get process name, with fallback
                let proc =
                    process_name(&conn, win, net_wm_pid).unwrap_or_else(|_| "<unknown>".into());

                // Extract and save the window icon
                let icon_data = get_icon_data(&conn, win, net_wm_icon);

                // Convert icon data to base64 encoded PNG
                let icon_base64 = match &icon_data {
                    Ok(data) => convert_icon_to_base64(data).unwrap_or_else(|e| {
                        eprintln!("Failed to convert icon to base64: {}", e);
                        String::new() // Empty string if conversion fails
                    }),
                    Err(_) => String::new(), // Empty string if no icon data
                };

                eprintln!("▶ {proc}: {title}");

                if proc == "eur-tauri" {
                    // Skip the eur-tauri process itself
                    continue;
                }

                // Create a new activity for the focused window
                let mut s = String::from("");

                // Create a runtime to execute the async code
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(err) => {
                        eprintln!("Failed to create tokio runtime: {}", err);
                        continue;
                    }
                };

                // Use a match to handle potential errors in the async block
                match rt.block_on(async {
                    // Select the appropriate strategy based on the process name
                    let strategy_result = select_strategy_for_process(
                        &proc,
                        format!("{}: {}", proc, title),
                        icon_base64.clone(),
                    )
                    .await;

                    match strategy_result {
                        Ok(strategy) => {
                            timeline.start_collection_activity(strategy, &mut s).await;
                            Ok(())
                        }
                        Err(err) => Err(err),
                    }
                }) {
                    Ok(_) => (), // Successfully processed
                    Err(err) => eprintln!("Error processing window activity: {}", err),
                }
            }
        }
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

// The strategy selection logic has been moved to the eur_activity crate

// / Extract the window icon and save it to a file
// fn extract_and_save_icon<C: Connection>(
//     conn: &C,
//     window: u32,
//     net_wm_icon: u32,
//     proc_name: &str,
// ) -> Result<String> {
//     // Create directory for icons if it doesn't exist
//     let icon_dir = Path::new("./icons");
//     create_dir_all(icon_dir).context("Failed to create icons directory")?;

//     // Generate a unique filename for this process
//     let sanitized_name = proc_name.replace("/", "_").replace("\\", "_");
//     let icon_path = icon_dir.join(format!("{}.png", sanitized_name));

//     // Check if we already have this icon
//     if icon_path.exists() {
//         return Ok(icon_path.to_string_lossy().into_owned());
//     }

//     let icon_data = get_icon_data(conn, window, net_wm_icon)?;

//     if icon_data.len() < 2 {
//         return Err(anyhow::anyhow!("Invalid icon data"));
//     }

//     let width = icon_data[0] as u32;
//     let height = icon_data[1] as u32;

//     if width == 0 || height == 0 || width > 1024 || height > 1024 {
//         return Err(anyhow::anyhow!("Invalid icon dimensions"));
//     }

//     // Create an image buffer
//     let mut img = ImageBuffer::new(width, height);

//     // Fill the image with the icon data
//     for y in 0..height {
//         for x in 0..width {
//             let idx = 2 + (y * width + x) as usize;
//             if idx < icon_data.len() {
//                 let argb = icon_data[idx];
//                 let a = ((argb >> 24) & 0xFF) as u8;
//                 let r = ((argb >> 16) & 0xFF) as u8;
//                 let g = ((argb >> 8) & 0xFF) as u8;
//                 let b = (argb & 0xFF) as u8;
//                 img.put_pixel(x, y, Rgba([r, g, b, a]));
//             }
//         }
//     }

//     // Save the image to a file
//     img.save(&icon_path).context("Failed to save icon image")?;

//     Ok(icon_path.to_string_lossy().into_owned())
// }
