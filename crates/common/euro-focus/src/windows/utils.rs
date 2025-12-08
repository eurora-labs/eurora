use crate::{FerrousFocusError, FerrousFocusResult};
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

/// Get the handle of the currently focused window
pub fn get_foreground_window() -> Option<HWND> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        None
    } else {
        Some(hwnd)
    }
}

/// Check if we're running in an interactive session
pub fn is_interactive_session() -> FerrousFocusResult<bool> {
    // Check if we can get the foreground window
    // In a service context, this will typically fail
    Ok(unsafe { !GetForegroundWindow().is_null() })
}

/// Get the title of a window
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer (HWND).
/// The caller must ensure that the HWND is valid.
pub unsafe fn get_window_title(hwnd: HWND) -> FerrousFocusResult<String> {
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

/// Get the process ID of a window
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer (HWND).
/// The caller must ensure that the HWND is valid.
pub unsafe fn get_window_process_id(hwnd: HWND) -> FerrousFocusResult<u32> {
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
    }

    if process_id == 0 {
        return Err(FerrousFocusError::Platform(
            "Failed to get process ID".to_string(),
        ));
    }

    Ok(process_id)
}

/// Get the process name from a process ID
pub fn get_process_name(process_id: u32) -> FerrousFocusResult<String> {
    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, process_id) };

    if process_handle.is_null() {
        return Err(FerrousFocusError::Platform(
            "Failed to open process".to_string(),
        ));
    }

    let mut buffer = [0u16; 512];
    let len = unsafe {
        GetModuleBaseNameW(
            process_handle,
            std::ptr::null_mut(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        )
    };

    // Close the process handle
    unsafe {
        CloseHandle(process_handle);
    }

    if len == 0 {
        return Err(FerrousFocusError::Platform(
            "Failed to get module name".to_string(),
        ));
    }

    let name = OsString::from_wide(&buffer[..len as usize])
        .to_string_lossy()
        .into_owned();

    Ok(name)
}

/// Get window information (title and process name) for a given window handle
///
/// # Safety
/// This function is unsafe because it calls unsafe functions that dereference raw pointers.
/// The caller must ensure that the HWND is valid.
pub unsafe fn get_window_info(hwnd: HWND) -> FerrousFocusResult<(String, String)> {
    let title = unsafe { get_window_title(hwnd) }.unwrap_or_else(|_| String::new());
    let process_id = unsafe { get_window_process_id(hwnd) }?;
    let process_name =
        get_process_name(process_id).unwrap_or_else(|_| format!("Process_{}", process_id));

    Ok((title, process_name))
}

/// Check if a window handle is valid
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer (HWND).
/// The caller must ensure that the HWND is not dangling.
pub unsafe fn is_valid_window(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindow(hwnd) } != 0
}
