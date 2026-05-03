use cfg_if::cfg_if;

mod app_process;
mod browser;

pub use app_process::AppProcess;
pub use browser::{Browser, BrowserStore};

#[inline(always)]
pub fn os_pick<'a>(_windows: &'a str, _linux: &'a str, _macos: &'a str) -> &'a str {
    cfg_if! {
        if #[cfg(target_os = "windows")] { _windows }
        else if #[cfg(target_os = "linux")] { _linux }
        else if #[cfg(target_os = "macos")] { _macos }
        else { compile_error!("Unsupported target OS"); }
    }
}

/// Compare a process name declared at compile time against a focused-window
/// report from the OS.
///
/// On Windows the comparison is ASCII case-insensitive because the focus
/// tracker reports executable names with inconsistent casing there. On other
/// targets the comparison is byte-exact.
#[cfg(target_os = "windows")]
pub fn process_name_matches(known: &str, candidate: &str) -> bool {
    known.eq_ignore_ascii_case(candidate)
}

#[cfg(not(target_os = "windows"))]
pub fn process_name_matches(known: &str, candidate: &str) -> bool {
    known == candidate
}
