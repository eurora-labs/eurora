use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

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

/// Fired after a freshly-tracked activity has been persisted via
/// `POST /activities`. Carries the persisted `id` (matches the server
/// row) plus everything the frontend needs to render the activity in the
/// timeline rail without re-fetching — the rail subscribes and prepends
/// optimistically, the server has already accepted the row.
#[derive(Debug, Clone)]
pub struct SavedActivityEvent {
    pub id: Uuid,
    pub name: String,
    pub process_name: String,
    pub window_title: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub icon: Option<Arc<image::RgbaImage>>,
}
