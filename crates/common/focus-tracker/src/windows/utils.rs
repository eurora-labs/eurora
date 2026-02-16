use crate::{FocusTrackerError, FocusTrackerResult};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND},
    System::{
        ProcessStatus::GetModuleBaseNameW,
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
    UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsWindow,
    },
};

pub fn get_foreground_window() -> Option<HWND> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        None
    } else {
        Some(hwnd)
    }
}

pub fn is_interactive_session() -> FocusTrackerResult<bool> {
    use windows_sys::Win32::System::StationsAndDesktops::{
        GetProcessWindowStation, GetUserObjectInformationW, UOI_FLAGS, USEROBJECTFLAGS,
    };

    let station = unsafe { GetProcessWindowStation() };
    if station.is_null() {
        return Err(FocusTrackerError::platform(
            "failed to get process window station",
        ));
    }

    let mut flags: USEROBJECTFLAGS = unsafe { std::mem::zeroed() };
    let mut needed: u32 = 0;
    let ok = unsafe {
        GetUserObjectInformationW(
            station as _,
            UOI_FLAGS,
            &mut flags as *mut _ as *mut _,
            std::mem::size_of::<USEROBJECTFLAGS>() as u32,
            &mut needed,
        )
    };

    if ok == 0 {
        return Err(FocusTrackerError::platform(
            "failed to get window station flags",
        ));
    }

    Ok(flags.dwFlags & 1 != 0)
}

pub(crate) fn get_window_title(hwnd: HWND) -> FocusTrackerResult<String> {
    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };

    if len == 0 {
        return Ok(String::new());
    }

    let title = OsString::from_wide(&buffer[..len as usize])
        .to_string_lossy()
        .into_owned();

    Ok(title)
}

pub(crate) fn get_window_process_id(hwnd: HWND) -> FocusTrackerResult<u32> {
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
    }

    if process_id == 0 {
        return Err(FocusTrackerError::platform("failed to get process ID"));
    }

    Ok(process_id)
}

pub(crate) fn get_process_name(process_id: u32) -> FocusTrackerResult<String> {
    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, process_id) };

    if process_handle.is_null() {
        return Err(FocusTrackerError::platform("failed to open process"));
    }

    struct HandleGuard(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }
    let _guard = HandleGuard(process_handle);

    let mut buffer = [0u16; 512];
    let len = unsafe {
        GetModuleBaseNameW(
            process_handle,
            std::ptr::null_mut(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        )
    };

    if len == 0 {
        return Err(FocusTrackerError::platform("failed to get module name"));
    }

    let name = OsString::from_wide(&buffer[..len as usize])
        .to_string_lossy()
        .into_owned();

    Ok(name)
}

pub(crate) fn get_process_exe_path(process_id: u32) -> FocusTrackerResult<Vec<u16>> {
    use windows_sys::Win32::System::Threading::{PROCESS_NAME_WIN32, QueryFullProcessImageNameW};

    let process_handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, 0, process_id) };

    if process_handle.is_null() {
        return Err(FocusTrackerError::platform("failed to open process"));
    }

    struct HandleGuard(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }
    let _guard = HandleGuard(process_handle);

    let mut buffer = vec![0u16; 32768];
    let mut len = buffer.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(
            process_handle,
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &mut len,
        )
    };

    if ok == 0 || len == 0 {
        return Err(FocusTrackerError::platform(
            "failed to query process image name",
        ));
    }

    buffer.truncate(len as usize);
    Ok(buffer)
}

pub(crate) fn get_window_info(hwnd: HWND) -> FocusTrackerResult<(Option<String>, String)> {
    let title = get_window_title(hwnd).unwrap_or_default();
    let title = if title.is_empty() { None } else { Some(title) };
    let process_id = get_window_process_id(hwnd)?;
    let process_name =
        get_process_name(process_id).unwrap_or_else(|_| format!("Process_{}", process_id));

    Ok((title, process_name))
}
