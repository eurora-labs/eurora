use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ActivityEvent {
    /// Display name of the focused activity. For browser activities this is
    /// the page URL; for other apps it is the window title.
    pub name: String,
    /// Executable name of the focused process. Stable identifier suitable
    /// for matching against `euro_process` browser definitions.
    pub process_name: String,
    /// OS-level process id of the focused process. Used by clients that
    /// need to address the process directly (e.g. opening a URL in the
    /// same browser instance).
    pub process_id: u32,
    pub icon: Option<Arc<image::RgbaImage>>,
}
