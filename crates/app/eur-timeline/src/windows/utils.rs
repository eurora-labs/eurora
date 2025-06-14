use anyhow::Result;
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

/// Get the title of a window
pub fn get_window_title(hwnd: HWND) -> Result<String> {
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
pub fn get_window_process_id(hwnd: HWND) -> Result<u32> {
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
    }
    
    if process_id == 0 {
        return Err(anyhow::anyhow!("Failed to get process ID"));
    }
    
    Ok(process_id)
}

/// Get the process name from a process ID
pub fn get_process_name(process_id: u32) -> Result<String> {
    let process_handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            0,
            process_id,
        )
    };
    
    if process_handle.is_null() {
        return Err(anyhow::anyhow!("Failed to open process"));
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
        return Err(anyhow::anyhow!("Failed to get module name"));
    }
    
    let name = OsString::from_wide(&buffer[..len as usize])
        .to_string_lossy()
        .into_owned();
    
    Ok(name)
}

/// Get window information (title and process name) for a given window handle
pub fn get_window_info(hwnd: HWND) -> Result<(String, String)> {
    let title = get_window_title(hwnd).unwrap_or_else(|_| String::new());
    let process_id = get_window_process_id(hwnd)?;
    let process_name = get_process_name(process_id)
        .unwrap_or_else(|_| format!("Process_{}", process_id));
    
    Ok((title, process_name))
}

/// Check if a window handle is valid
pub fn is_valid_window(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindow(hwnd) } != 0
}