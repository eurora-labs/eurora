use anyhow::Result;
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
    wrapper::ConnectionExt as _,
};

pub fn spawn() {
    // Run in its own thread so the Tauri event‑loop stays free.
    thread::spawn(|| {
        if let Err(e) = track_focus() {
            eprintln!("Focus‑tracker exited: {e}");
        }
    });
}

fn track_focus() -> Result<()> {
    // Connect to the X server
    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root_window_id = screen.root;

    // Atoms we’ll need
    let net_active_window = atom(&conn, b"_NET_ACTIVE_WINDOW")?;
    let net_wm_name = atom(&conn, b"_NET_WM_NAME")?;
    let net_wm_pid = atom(&conn, b"_NET_WM_PID")?;
    let utf8_string = atom(&conn, b"UTF8_STRING")?;

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
                    println!("▶ {proc}: {title}");
                    // if let Ok(name) = window_name(&conn, win, net_wm_name, utf8_string) {
                    //     println!("▶ Currently focused: {name}");
                    // }
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
