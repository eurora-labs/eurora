/// Snapshot of the currently focused window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusedWindow {
    /// Process ID of the focused window.
    pub process_id: String,
    /// Reported process name, if not available nothing is reported.
    pub process_name: String,
    /// Full window title/caption as provided by the OS.
    pub window_title: Option<String>,
    /// Raw icon data (may be `None` if not retrievable on the platform).
    pub icon: Option<image::RgbaImage>,
}
