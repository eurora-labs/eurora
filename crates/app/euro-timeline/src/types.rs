use std::sync::Arc;

use activity_core::{Activity as WireActivity, ActivitySession as WireActivitySession};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ActivityEvent {
    /// Display name of the focused activity — `display_name` from the
    /// parent activity row (e.g. `"Youtube"`, `"Code"`).
    pub name: String,
    /// Executable name of the focused process. Stable identifier
    /// suitable for matching against `euro_process` browser definitions.
    pub process_name: String,
    /// OS-level process id of the focused process. Used by clients that
    /// need to address the process directly (e.g. opening a URL in the
    /// same browser instance).
    pub process_id: u32,
    pub icon: Option<Arc<image::RgbaImage>>,
}

/// Fired after a session has been persisted via `POST /activity-sessions`.
///
/// Carries the (possibly upserted) parent activity *and* the new session
/// in one atomic payload so the rail can render "Youtube is live now"
/// without waiting for a follow-up event or refetching the list. The
/// parent's `last_used_at` was just advanced server-side, so a re-sort
/// on the frontend will lift it to position 0.
#[derive(Debug, Clone)]
pub struct SavedActivityEvent {
    pub activity: WireActivity,
    pub session: WireActivitySession,
    /// Decoded PNG bytes for the activity icon — the desktop's Tauri
    /// layer needs them to compute an accent colour and produce a
    /// `data:` URL without a follow-up HTTP fetch through the asset
    /// service.
    pub icon: Option<Arc<image::RgbaImage>>,
}

/// Fired after a session's closing PATCH (`ended_at` transition from
/// NULL to a real timestamp) has been accepted by the server.
///
/// Lets the rail strip the live indicator from the parent in place —
/// without it the frontend would keep treating the activity as live
/// until the next focus event arrived for a different parent.
#[derive(Debug, Clone)]
pub struct SavedActivityEndedEvent {
    pub activity_id: Uuid,
    pub session_id: Uuid,
    pub ended_at: DateTime<Utc>,
}
