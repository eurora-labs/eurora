//! Resolve a PID to its executable name on the current OS.
//!
//! Per-OS implementations:
//!
//! - **Linux** reads `/proc/{pid}/comm`.
//! - **macOS** calls `proc_pidpath(2)` and returns the file-name component.
//! - **Windows** walks the ToolHelp32 process snapshot until it finds a
//!   matching PID and returns the UTF-16 executable name.
//!
//! Returns [`None`] for any failure mode (process gone, permission denied,
//! empty name) so callers can fall back without distinguishing causes.

/// Look up the executable name of the process identified by `pid`.
pub fn lookup_process_name(pid: u32) -> Option<String> {
    lookup_process_name_impl(pid)
}

#[cfg(target_os = "windows")]
fn lookup_process_name_impl(pid: u32) -> Option<String> {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct PROCESSENTRY32W {
        dwSize: u32,
        cntUsage: u32,
        th32ProcessID: u32,
        th32DefaultHeapID: usize,
        th32ModuleID: u32,
        cntThreads: u32,
        th32ParentProcessID: u32,
        pcPriClassBase: i32,
        dwFlags: u32,
        szExeFile: [u16; 260],
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
        fn Process32FirstW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        fn Process32NextW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        fn CloseHandle(hObject: isize) -> i32;
    }

    const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    const INVALID_HANDLE_VALUE: isize = -1;

    // SAFETY: All FFI calls match the documented Win32 signatures.
    // `entry` is zero-initialized (the struct is plain-data with no
    // niches) and `dwSize` is set to its declared size before the
    // first ToolHelp call, as the API requires.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) == 0 {
            CloseHandle(snapshot);
            return None;
        }

        loop {
            if entry.th32ProcessID == pid {
                let name_len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);
                CloseHandle(snapshot);
                return Some(name);
            }

            if Process32NextW(snapshot, &mut entry) == 0 {
                break;
            }
        }

        CloseHandle(snapshot);
    }

    None
}

#[cfg(target_os = "linux")]
fn lookup_process_name_impl(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "macos")]
fn lookup_process_name_impl(pid: u32) -> Option<String> {
    unsafe extern "C" {
        fn proc_pidpath(pid: i32, buffer: *mut u8, buffersize: u32) -> i32;
    }
    let mut buf = [0u8; 4096];
    // SAFETY: `proc_pidpath` writes at most `buf.len()` bytes into the
    // provided buffer and returns the number written, or a non-positive
    // value on failure.
    let ret = unsafe { proc_pidpath(pid as i32, buf.as_mut_ptr(), buf.len() as u32) };
    if ret > 0 {
        std::str::from_utf8(&buf[..ret as usize])
            .ok()
            .and_then(|path| path.rsplit('/').next())
            .map(|s| s.to_string())
    } else {
        None
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn lookup_process_name_impl(_pid: u32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_current_process() {
        let pid = std::process::id();
        let name = lookup_process_name(pid).expect("self pid resolves");
        assert!(!name.is_empty(), "process name should not be empty");
    }

    #[test]
    fn unknown_pid_returns_none() {
        // `u32::MAX` is reserved on every supported OS and will not match
        // any live process; the lookup must return `None`.
        assert_eq!(lookup_process_name(u32::MAX), None);
    }
}
