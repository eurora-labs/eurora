pub fn get_process_name(pid: u32) -> Option<String> {
    get_process_name_impl(pid)
}

#[cfg(target_os = "windows")]
fn get_process_name_impl(pid: u32) -> Option<String> {
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

    const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    const INVALID_HANDLE_VALUE: isize = -1;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut entry: MaybeUninit<PROCESSENTRY32W> = MaybeUninit::uninit();
        (*entry.as_mut_ptr()).dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, entry.as_mut_ptr()) == 0 {
            CloseHandle(snapshot);
            return None;
        }

        loop {
            let entry_ref = entry.assume_init_ref();
            if entry_ref.th32ProcessID == pid {
                let name_raw = &entry_ref.szExeFile;
                let name_len = name_raw
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(name_raw.len());
                let name = String::from_utf16_lossy(&name_raw[..name_len]);
                CloseHandle(snapshot);
                return Some(name);
            }

            if Process32NextW(snapshot, entry.as_mut_ptr()) == 0 {
                break;
            }
        }

        CloseHandle(snapshot);
    }

    None
}

#[cfg(target_os = "linux")]
fn get_process_name_impl(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "macos")]
fn get_process_name_impl(pid: u32) -> Option<String> {
    unsafe extern "C" {
        fn proc_pidpath(pid: i32, buffer: *mut u8, buffersize: u32) -> i32;
    }
    let mut buf = [0u8; 4096];
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
