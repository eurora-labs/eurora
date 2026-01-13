mod config;
mod error;
mod focus_window;

pub use config::{FocusTrackerConfig, IconConfig};
pub use error::{FocusTrackerError, FocusTrackerResult};
pub use focus_window::FocusedWindow;
