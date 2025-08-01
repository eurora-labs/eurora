mod types;

pub use types::*;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

mod active_monitor;
pub use active_monitor::ActiveMonitor;
