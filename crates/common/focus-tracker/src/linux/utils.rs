use std::env::var_os;

/// Checks if Wayland based on two common variables:
/// - `XDG_SESSION_TYPE` — set by the display manager, compared case-insensitively
/// - `WAYLAND_DISPLAY` — set by the compositor when a Wayland socket is available;
pub fn wayland_detect() -> bool {
    let is_wayland_session = var_os("XDG_SESSION_TYPE")
        .map(|v| v.to_string_lossy().eq_ignore_ascii_case("wayland"))
        .unwrap_or(false);

    let has_wayland_display = var_os("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    is_wayland_session || has_wayland_display
}
