//! Walk the process ancestor chain to find the browser that hosts our
//! native-messaging stdio process.
//!
//! Each browser launches the native host as a child (Linux/macOS) or as
//! a descendant after a launcher process (Windows). We want the browser
//! PID, not our own — the desktop bridge keys registered clients by it.
//!
use crate::Browser;

/// Direct parent process ID.
///
/// Returns `0` on platforms that don't expose `getppid`-equivalent
/// semantics. All currently supported targets (Linux, macOS, Windows)
/// do.
pub fn parent_pid() -> u32 {
    parent_pid_impl()
}

/// Walk our ancestor chain looking for a known browser executable.
///
/// Resolution order:
///
/// 1. Direct parent — the common case on Linux/macOS where the browser
///    `exec`s the native host with itself as parent.
/// 2. On Linux: a sibling of the direct parent (a launcher process can
///    sit between the browser and the host; the browser is then the
///    grandparent's other child).
/// 3. On Windows: any ancestor — Chrome on Windows interposes
///    `chrome.exe --type=utility` between the user-launched browser and
///    the native host.
///
/// Returns `None` if no ancestor matches a known browser, in which case
/// the caller should fall back to [`parent_pid`].
pub fn browser_ancestor_pid() -> Option<u32> {
    browser_ancestor_pid_impl()
}

#[cfg(target_os = "linux")]
fn parent_pid_impl() -> u32 {
    std::os::unix::process::parent_id()
}

#[cfg(target_os = "macos")]
fn parent_pid_impl() -> u32 {
    std::os::unix::process::parent_id()
}

#[cfg(target_os = "windows")]
fn parent_pid_impl() -> u32 {
    use std::process;

    process_table()
        .get(&process::id())
        .map(|(ppid, _)| *ppid)
        .unwrap_or(0)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn parent_pid_impl() -> u32 {
    0
}

#[cfg(target_os = "linux")]
fn browser_ancestor_pid_impl() -> Option<u32> {
    let browsers = known_browser_names();
    let direct = std::os::unix::process::parent_id();

    if is_browser_process(direct, &browsers) {
        return Some(direct);
    }

    let (grandparent, _) = read_proc_stat(direct)?;
    if grandparent <= 1 {
        return None;
    }

    find_browser_child(grandparent, &browsers)
}

#[cfg(target_os = "linux")]
fn is_browser_process(pid: u32, browsers: &[&str]) -> bool {
    read_proc_stat(pid).is_some_and(|(_, name)| browsers.iter().any(|b| name == *b))
}

#[cfg(target_os = "linux")]
fn find_browser_child(parent: u32, browsers: &[&str]) -> Option<u32> {
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
            && ppid == parent
            && browsers.iter().any(|b| name == *b)
        {
            return Some(pid);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn read_proc_stat(pid: u32) -> Option<(u32, String)> {
    // `/proc/{pid}/stat` format: `pid (comm) state ppid …`. The `comm`
    // field is parenthesised because process names can contain spaces;
    // we scan for the *last* `)` so embedded `)` characters in the name
    // don't truncate the parse.
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let comm_start = stat.find('(')? + 1;
    let comm_end = stat.rfind(')')?;
    let comm = stat[comm_start..comm_end].to_string();
    let rest = &stat[comm_end + 2..];
    let ppid: u32 = rest.split_whitespace().nth(1)?.parse().ok()?;
    Some((ppid, comm))
}

#[cfg(target_os = "macos")]
fn browser_ancestor_pid_impl() -> Option<u32> {
    // macOS browsers launch native hosts as direct children of the
    // browser binary; no launcher process sits in between. If the
    // direct parent isn't a browser there's nowhere else to look.
    let browsers = known_browser_names();
    let parent = std::os::unix::process::parent_id();
    let name = crate::lookup_process_name(parent)?;
    browsers.iter().any(|b| name == *b).then_some(parent)
}

#[cfg(target_os = "windows")]
fn browser_ancestor_pid_impl() -> Option<u32> {
    use std::collections::HashSet;
    use std::process;

    let browsers = known_browser_names();
    let table = process_table();
    let mut current = process::id();
    let mut visited = HashSet::new();

    while let Some(&(parent, ref name)) = table.get(&current) {
        if parent == 0 || !visited.insert(current) {
            break;
        }
        if browsers.iter().any(|b| b.eq_ignore_ascii_case(name)) {
            return Some(current);
        }
        current = parent;
    }

    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn browser_ancestor_pid_impl() -> Option<u32> {
    None
}

/// Browsers we expect to see hosting the native-messaging child.
///
/// The wider catalog in [`Browser::ALL`] covers every browser Eurora
/// can identify by focused-window name, but only a few launch native
/// hosts directly today. Keeping the ancestor walk's match set small
/// avoids paying a `/proc` scan cost (Linux) or a snapshot scan
/// (Windows) for browsers that never reach this code.
fn known_browser_names() -> [&'static str; 3] {
    [
        Browser::Firefox.process_name(),
        Browser::Chrome.process_name(),
        Browser::Librewolf.process_name(),
    ]
}

#[cfg(target_os = "windows")]
fn process_table() -> std::collections::HashMap<u32, (u32, String)> {
    use std::collections::HashMap;
    use std::mem::MaybeUninit;

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

    fn exe_name(entry: &PROCESSENTRY32W) -> String {
        let raw = &entry.szExeFile;
        let len = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        String::from_utf16_lossy(&raw[..len])
    }

    const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    const INVALID_HANDLE_VALUE: isize = -1;

    let mut table: HashMap<u32, (u32, String)> = HashMap::new();

    // SAFETY: `entry.dwSize` is set before the first ToolHelp call as
    // the API requires; `Process32FirstW` / `Process32NextW` populate
    // the rest before we read it via `assume_init_ref`.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return table;
        }

        let mut entry: MaybeUninit<PROCESSENTRY32W> = MaybeUninit::uninit();
        (*entry.as_mut_ptr()).dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, entry.as_mut_ptr()) != 0 {
            loop {
                let e = entry.assume_init_ref();
                table.insert(e.th32ProcessID, (e.th32ParentProcessID, exe_name(e)));

                if Process32NextW(snapshot, entry.as_mut_ptr()) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_pid_is_positive() {
        let ppid = parent_pid();
        assert!(ppid > 0, "expected positive parent PID, got {ppid}");
    }

    #[test]
    fn browser_ancestor_pid_does_not_panic() {
        // Outcome (`Some(pid)` or `None`) depends on the test runner's
        // ancestor chain; we only assert the FFI/parse path doesn't trap.
        let _ = browser_ancestor_pid();
    }
}
