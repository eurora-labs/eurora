pub use focus_tracker_core::*;

mod focus_tracker;
pub(crate) mod icon_cache;

pub use focus_tracker::*;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform;

/// Returns the document URL of the focused window for the given OS process,
/// when the platform and the application expose one.
///
/// This is a thin facade over the platform module. Today only macOS provides
/// a real implementation (via the Accessibility API's `AXDocument`
/// attribute); Linux and Windows return `Ok(None)` until equivalent support
/// is implemented for those targets.
///
/// Callers should treat `Ok(None)` as a soft signal ("no document open / no
/// URL available") and surface real errors only in the
/// [`FocusTrackerError::PermissionDenied`] case, which means the platform
/// blocked the lookup and the caller should back off.
///
/// # Errors
///
/// Returns [`FocusTrackerError::PermissionDenied`] when macOS denies
/// Accessibility access. Other errors are platform-specific.
pub fn focused_document_url(pid: u32) -> FocusTrackerResult<Option<String>> {
    platform::focused_document_url(pid)
}
