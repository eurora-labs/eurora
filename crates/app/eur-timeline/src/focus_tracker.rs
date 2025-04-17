use anyhow::{Context, Result};
use image::{ImageBuffer, Rgba};
use std::fs::create_dir_all;
use std::path::Path;
use std::thread;

use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ConnectionExt, EventMask, GetPropertyReply,
            PropertyNotifyEvent,
        },
    },
    rust_connection::RustConnection,
};

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

fn track_focus(timeline: super::TimelineRef) -> Result<()> {
    // Connect to the X server
    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root_window_id = screen.root;

    // Atoms we’ll need
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
        let event = conn.wait_for_event()?; // blocking
        if let Event::PropertyNotify(PropertyNotifyEvent { atom, .. }) = event {
            // Has the active window changed?
            if atom == net_active_window {
                if let Some(win) = active_window(&conn, root_window_id, net_active_window)? {
                    let title = window_name(&conn, win, net_wm_name, utf8_string)?;
                    let proc =
                        process_name(&conn, win, net_wm_pid).unwrap_or_else(|_| "<unknown>".into());

                    // Extract and save the window icon
                    let icon_data = get_icon_data(&conn, win, net_wm_icon);
                    // let icon_path = match extract_and_save_icon(&conn, win, net_wm_icon, &proc) {
                    //     Ok(path) => path,
                    //     Err(e) => {
                    //         eprintln!("Failed to extract icon: {}", e);
                    //         // Use a default icon path or just the process name if icon extraction fails
                    //         proc.clone()
                    //     }
                    // };

                    // Create a new activity for the focused window
                    let activity_name = format!("{}: {}", proc, title);
                    let activity = super::Activity::new(
                        activity_name,
                        // icon_path, // Use the path to the saved icon
                        "".into(), // Use the path to the saved icon
                        super::ActivityType::Application,
                    );

                    // Add the activity to the timeline
                    timeline.add_activity(activity);
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
    let reply = conn
        .get_property(false, root, net_active_window, AtomEnum::WINDOW, 0, 1)?
        .reply()?;
    Ok(reply.value32().and_then(|mut v| v.next()))
}

fn window_name<C: Connection>(
    conn: &C,
    window: u32,
    net_wm_name: u32,
    utf8_string: u32,
) -> Result<String> {
    // Try UTF‑8 first
    let reply: GetPropertyReply = conn
        .get_property(false, window, net_wm_name, utf8_string, 0, u32::MAX)?
        .reply()?;
    if reply.value_len > 0 {
        return Ok(String::from_utf8_lossy(&reply.value).into_owned());
    }

    // Fallback to the legacy WM_NAME
    let reply: GetPropertyReply = conn
        .get_property(
            false,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            0,
            u32::MAX,
        )?
        .reply()?;
    Ok(String::from_utf8_lossy(&reply.value).into_owned())
}

fn process_name<C: Connection>(conn: &C, window: u32, net_wm_pid: u32) -> Result<String> {
    // fetch the PID stored in _NET_WM_PID
    let reply: GetPropertyReply = conn
        .get_property(false, window, net_wm_pid, AtomEnum::CARDINAL, 0, 1)?
        .reply()?;
    let pid = reply.value32().and_then(|mut v| v.next()).unwrap();

    // read /proc/<pid>/comm  (single line: executable name)
    let name = std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .or_else(|_| {
            std::fs::read_link(format!("/proc/{pid}/exe")).map(|p| p.to_string_lossy().into())
        })
        .unwrap();

    Ok(name.trim_end_matches('\n').to_owned())
}

fn get_icon_data<C: Connection>(conn: &C, window: u32, net_wm_icon: u32) -> Result<Vec<u32>> {
    let reply = conn
        .get_property(
            false,
            window,
            net_wm_icon,
            AtomEnum::CARDINAL,
            0,
            u32::MAX / 4, // Limit size to avoid huge icons
        )?
        .reply()?;

    if reply.value_len == 0 {
        return Err(anyhow::anyhow!("No icon data available"));
    }

    // The icon data is an array of 32-bit values
    Ok(reply.value32().unwrap().collect())
}

/// Extract the window icon and save it to a file
fn extract_and_save_icon<C: Connection>(
    conn: &C,
    window: u32,
    net_wm_icon: u32,
    proc_name: &str,
) -> Result<String> {
    // Create directory for icons if it doesn't exist
    let icon_dir = Path::new("./icons");
    create_dir_all(icon_dir).context("Failed to create icons directory")?;

    // Generate a unique filename for this process
    let sanitized_name = proc_name.replace("/", "_").replace("\\", "_");
    let icon_path = icon_dir.join(format!("{}.png", sanitized_name));

    // Check if we already have this icon
    if icon_path.exists() {
        return Ok(icon_path.to_string_lossy().into_owned());
    }

    let icon_data = get_icon_data(conn, window, net_wm_icon)?;

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

    // Save the image to a file
    img.save(&icon_path).context("Failed to save icon image")?;

    Ok(icon_path.to_string_lossy().into_owned())
}
