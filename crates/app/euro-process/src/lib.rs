mod ancestry;
mod app_process;
mod browser;
mod process_name;

pub use ancestry::{browser_ancestor_pid, parent_pid};
pub use app_process::AppProcess;
pub use browser::{Browser, BrowserStore};
pub use process_name::lookup_process_name;

#[inline(always)]
pub fn os_pick<'a>(_windows: &'a str, _linux: &'a str, _macos: &'a str) -> &'a str {
    cfg_select! {
        target_os = "windows" => { _windows }
        target_os = "linux" => { _linux }
        target_os = "macos" => { _macos }
        _ => { compile_error!("Unsupported target OS") }
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
