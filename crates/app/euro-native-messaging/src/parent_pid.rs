use std::sync::OnceLock;

static PARENT_PID: OnceLock<u32> = OnceLock::new();

/// On Safari/macOS, the Swift bridge passes the actual Safari PID via
/// EURORA_BROWSER_PID since the native messaging host's parent would be
/// the Swift bridge app, not Safari.
pub fn capture_parent_pid() {
    let ppid = if let Ok(env_pid) = std::env::var("EURORA_BROWSER_PID") {
        if let Ok(pid) = env_pid.parse::<u32>() {
            tracing::info!(
                "Using browser PID from EURORA_BROWSER_PID environment variable: {}",
                pid
            );
            pid
        } else {
            tracing::warn!(
                "Invalid EURORA_BROWSER_PID value '{}', falling back to parent PID",
                env_pid
            );
            get_parent_pid_impl()
        }
    } else {
        get_parent_pid_impl()
    };

    if PARENT_PID.set(ppid).is_err() {
        tracing::warn!("Parent PID was already captured");
    }
    tracing::info!("Captured browser PID: {}", ppid);
}

pub fn get_parent_pid() -> u32 {
    *PARENT_PID.get().unwrap_or(&0)
}

#[cfg(target_os = "linux")]
fn get_parent_pid_impl() -> u32 {
    use std::os::unix::process::parent_id;
    parent_id()
}

#[cfg(target_os = "macos")]
fn get_parent_pid_impl() -> u32 {
    use std::os::unix::process::parent_id;
    parent_id()
}

#[cfg(target_os = "windows")]
fn get_parent_pid_impl() -> u32 {
    use std::mem::MaybeUninit;
    use std::process;

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

    let current_pid = process::id();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            tracing::error!("Failed to create process snapshot");
            return 0;
        }

        let mut entry: MaybeUninit<PROCESSENTRY32W> = MaybeUninit::uninit();
        (*entry.as_mut_ptr()).dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, entry.as_mut_ptr()) == 0 {
            CloseHandle(snapshot);
            tracing::error!("Failed to get first process entry");
            return 0;
        }

        loop {
            let entry_ref = entry.assume_init_ref();
            if entry_ref.th32ProcessID == current_pid {
                let parent_pid = entry_ref.th32ParentProcessID;
                CloseHandle(snapshot);
                return parent_pid;
            }

            if Process32NextW(snapshot, entry.as_mut_ptr()) == 0 {
                break;
            }
        }

        CloseHandle(snapshot);
    }

    tracing::error!("Could not find current process in snapshot");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_parent_pid_impl() {
        let ppid = get_parent_pid_impl();
        // Parent PID should always be > 0 on any running system
        assert!(ppid > 0, "Parent PID should be greater than 0");
    }
}
