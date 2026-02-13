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

pub use platform::utils;

pub fn subscribe_focus_changes() -> FocusTrackerResult<FocusSubscription> {
    let tracker = FocusTracker::new();
    tracker.subscribe_focus_changes()
}
