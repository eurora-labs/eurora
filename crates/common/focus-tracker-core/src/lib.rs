mod config;
mod error;
mod focus_window;
mod ignored_processes;

pub use config::{FocusTrackerConfig, IconConfig};
pub use error::{FocusTrackerError, FocusTrackerResult};
pub use focus_window::FocusedWindow;
pub use ignored_processes::IgnoredProcesses;
