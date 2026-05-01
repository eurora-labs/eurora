//! Launch a URL inside a specific running browser instance.
//!
//! Tauri's `shell.open` always defers to the OS default browser. For the
//! "install Eurora extension" affordance we want the link to open inside the
//! browser the user is currently focused on, even when that browser is not
//! the system default. Every modern browser accepts a URL as a CLI argument
//! and reuses its running instance, so given the focused process's PID we
//! can resolve its executable (or app bundle, on macOS) and spawn the URL
//! into it.

use std::process::Command;

/// Open `url` inside the browser whose process id is `pid`.
///
/// Returns an error if the PID can no longer be resolved (process exited,
/// permission denied, etc.) or if the spawn itself fails. The frontend is
/// expected to fall back to the OS default browser in that case.
pub fn open_url_in_process(pid: u32, url: &str) -> Result<(), String> {
    if !is_safe_url(url) {
        return Err(format!("Refusing to open non-http(s) URL: {url}"));
    }

    open_url_in_process_impl(pid, url)
}

fn is_safe_url(url: &str) -> bool {
    matches!(url.split_once(':'), Some((scheme, _)) if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
}

#[cfg(target_os = "linux")]
fn open_url_in_process_impl(pid: u32, url: &str) -> Result<(), String> {
    let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
        .map_err(|e| format!("Failed to resolve executable for PID {pid}: {e}"))?;

    Command::new(&exe)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to spawn {}: {e}", exe.display()))
}

#[cfg(target_os = "macos")]
fn open_url_in_process_impl(pid: u32, url: &str) -> Result<(), String> {
    use std::path::PathBuf;

    unsafe extern "C" {
        fn proc_pidpath(pid: i32, buffer: *mut u8, buffersize: u32) -> i32;
    }

    let mut buf = [0u8; 4096];
    let ret = unsafe { proc_pidpath(pid as i32, buf.as_mut_ptr(), buf.len() as u32) };
    if ret <= 0 {
        return Err(format!("Failed to resolve executable for PID {pid}"));
    }

    let path_str = std::str::from_utf8(&buf[..ret as usize])
        .map_err(|e| format!("Executable path is not valid UTF-8: {e}"))?;
    let exe_path = PathBuf::from(path_str);

    let app_bundle = ancestor_app_bundle(&exe_path)
        .ok_or_else(|| format!("PID {pid} is not inside a .app bundle: {path_str}"))?;

    let status = Command::new("/usr/bin/open")
        .arg("-a")
        .arg(&app_bundle)
        .arg(url)
        .status()
        .map_err(|e| format!("Failed to invoke `open`: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`open -a {} {url}` exited with {status}",
            app_bundle.display()
        ))
    }
}

#[cfg(target_os = "macos")]
fn ancestor_app_bundle(exe: &std::path::Path) -> Option<std::path::PathBuf> {
    exe.ancestors()
        .find(|p| p.extension().is_some_and(|ext| ext == "app"))
        .map(|p| p.to_path_buf())
}

#[cfg(target_os = "windows")]
fn open_url_in_process_impl(pid: u32, url: &str) -> Result<(), String> {
    let exe = resolve_windows_exe_path(pid)
        .ok_or_else(|| format!("Failed to resolve executable for PID {pid}"))?;

    Command::new(&exe)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to spawn {}: {e}", exe.display()))
}

#[cfg(target_os = "windows")]
fn resolve_windows_exe_path(pid: u32) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
        fn CloseHandle(hObject: isize) -> i32;
        fn QueryFullProcessImageNameW(
            hProcess: isize,
            dwFlags: u32,
            lpExeName: *mut u16,
            lpdwSize: *mut u32,
        ) -> i32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return None;
        }

        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(handle);

        if ok == 0 || len == 0 {
            return None;
        }

        let path = String::from_utf16_lossy(&buf[..len as usize]);
        Some(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_urls() {
        assert!(!is_safe_url("file:///etc/passwd"));
        assert!(!is_safe_url("javascript:alert(1)"));
        assert!(!is_safe_url("not a url"));
    }

    #[test]
    fn accepts_http_urls() {
        assert!(is_safe_url("https://example.com"));
        assert!(is_safe_url("http://example.com"));
        assert!(is_safe_url("HTTPS://example.com"));
    }
}
