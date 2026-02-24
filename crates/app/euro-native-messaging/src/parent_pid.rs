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
    use euro_process::{Chrome, Firefox, Librewolf, ProcessFunctionality};
    use std::os::unix::process::parent_id;

    let browser_names: &[&str] = &[Firefox.get_name(), Chrome.get_name(), Librewolf.get_name()];

    let direct_ppid = parent_id();

    if is_browser_process(direct_ppid, browser_names) {
        return direct_ppid;
    }

    // The direct parent may be an intermediary (e.g. xdg-desktop-portal
    // when Firefox uses the freedesktop portal to launch native messaging
    // hosts). In that case the browser is a sibling of our parent, sharing
    // the same grandparent (typically systemd --user).
    if let Some((grandparent, _)) = read_proc_stat(direct_ppid)
        && grandparent > 1
        && let Some(browser_pid) = find_browser_child(grandparent, browser_names)
    {
        tracing::info!(
            "Found browser as sibling of parent: browser_pid={}, direct_parent={}, grandparent={}",
            browser_pid,
            direct_ppid,
            grandparent
        );
        return browser_pid;
    }

    tracing::debug!(
        "No browser found in process tree, using direct parent PID {}",
        direct_ppid
    );
    direct_ppid
}

#[cfg(target_os = "linux")]
fn is_browser_process(pid: u32, browser_names: &[&str]) -> bool {
    read_proc_stat(pid).is_some_and(|(_, name)| browser_names.iter().any(|b| name == *b))
}

/// Scans `/proc` for any process whose parent is `parent_pid` and whose
/// comm matches a known browser name.
#[cfg(target_os = "linux")]
fn find_browser_child(parent_pid: u32, browser_names: &[&str]) -> Option<u32> {
    let proc_dir = std::fs::read_dir("/proc").ok()?;
    for entry in proc_dir.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        if let Some((ppid, name)) = read_proc_stat(pid)
            && ppid == parent_pid
            && browser_names.iter().any(|b| name == *b)
        {
            return Some(pid);
        }
    }
    None
}

/// Reads `/proc/<pid>/stat` and returns `(ppid, comm)`.
#[cfg(target_os = "linux")]
fn read_proc_stat(pid: u32) -> Option<(u32, String)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Format: "<pid> (<comm>) <state> <ppid> ..."
    // comm can contain spaces and parentheses, so find the last ')'.
    let comm_start = stat.find('(')? + 1;
    let comm_end = stat.rfind(')')?;
    let comm = stat[comm_start..comm_end].to_string();
    let rest = &stat[comm_end + 2..];
    let ppid: u32 = rest.split_whitespace().nth(1)?.parse().ok()?;
    Some((ppid, comm))
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
        assert!(ppid > 0, "Parent PID should be greater than 0");
    }
}
