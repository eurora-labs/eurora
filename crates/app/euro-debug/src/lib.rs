#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use platform::detect_local_backend_endpoint;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn detect_local_backend_endpoint() -> Option<String> {
    None
}
