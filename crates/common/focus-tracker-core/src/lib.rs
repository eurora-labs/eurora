mod config;
mod error;
mod focus_window;
mod ignore_rules;

pub use config::{FocusTrackerConfig, IconConfig};
pub use error::{FocusTrackerError, FocusTrackerResult};
pub use focus_window::FocusedWindow;
pub use ignore_rules::{IgnoreRule, IgnoreRules, ProcessNameMatch, WindowTitleMatch};
