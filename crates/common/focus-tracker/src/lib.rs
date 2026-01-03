pub use focus_tracker_core::*;

mod focus_tracker;

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

// For platform specific util API's
pub use platform::utils;

/// Subscribe to focus changes and receive them via a channel
/// This is a convenience function that creates a new FocusTracker with default config and subscribes to changes
pub fn subscribe_focus_changes() -> FocusTrackerResult<std::sync::mpsc::Receiver<FocusedWindow>> {
    let tracker = FocusTracker::new();
    tracker.subscribe_focus_changes()
}
