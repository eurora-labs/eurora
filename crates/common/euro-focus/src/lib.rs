mod config;
mod error;
mod focus_tracker;
mod focused_window;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform;

pub use config::{FocusTrackerConfig, IconConfig};
pub use error::{FerrousFocusError, FerrousFocusResult};
pub use focus_tracker::FocusTracker;
pub use focused_window::FocusedWindow;

// For platform specific util API's
pub use platform::utils;

/// Subscribe to focus changes and receive them via a channel
/// This is a convenience function that creates a new FocusTracker with default config and subscribes to changes
pub fn subscribe_focus_changes() -> FerrousFocusResult<std::sync::mpsc::Receiver<FocusedWindow>> {
    let tracker = FocusTracker::new();
    tracker.subscribe_focus_changes()
}
