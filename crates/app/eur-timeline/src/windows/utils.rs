use std::ffi::{OsStr, OsString};
use std::mem;
use std::os::windows::ffi::OsStringExt;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{HWND, MAX_PATH, RECT};
use windows_sys::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows_sys::Win32::System::Threading::OpenProcess;
use windows_sys::Win32::System::Threading::PROCESS_QUERY_INFORMATION;
use windows_sys::Win32::System::Threading::PROCESS_VM_READ;
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsWindow,
};

/// Get the handle of the foreground window
pub fn get_foreground_window() -> Option<HWND> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd == 0 { None } else { Some(hwnd) }
}

/// Check if a window handle is valid
pub fn is_window_valid(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd) != 0 }
}

/// Get the title of a window
pub fn get_window_title(hwnd: HWND) -> Option<String> {
    if !is_window_valid(hwnd) {
        return None;
    }

    // First call to get the required buffer size
    let mut buffer = [0u16; MAX_PATH as usize];
    let len = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };

    if len == 0 {
        return None;
    }

    // Convert the buffer to a string
    let title = OsString::from_wide(&buffer[0..len as usize])
        .to_string_lossy()
        .into_owned();

    Some(title)
}

/// Get the process ID of a window
pub fn get_window_process_id(hwnd: HWND) -> Option<u32> {
    if !is_window_valid(hwnd) {
        return None;
    }

    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
    }

    if process_id == 0 {
        None
    } else {
        Some(process_id)
    }
}

/// Get the process name from a process ID
pub fn get_process_name(process_id: u32) -> Option<String> {
    // Open the process with query information and VM read access
    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, process_id) };

    if process_handle == 0 {
        return None;
    }

    // Get the process executable path
    let mut buffer = [0u16; MAX_PATH as usize];
    let len = unsafe {
        GetModuleFileNameExW(process_handle, 0, buffer.as_mut_ptr(), buffer.len() as u32)
    };

    // Close the process handle
    unsafe {
        windows_sys::Win32::Foundation::CloseHandle(process_handle);
    }

    if len == 0 {
        return None;
    }

    // Convert the buffer to a string
    let path = OsString::from_wide(&buffer[0..len as usize])
        .to_string_lossy()
        .into_owned();

    // Extract the filename from the path
    path.split('\\').last().map(|s| s.to_string())
}

/// Get the window rectangle
pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
    if !is_window_valid(hwnd) {
        return None;
    }

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    let result = unsafe { GetWindowRect(hwnd, &mut rect) };

    if result == 0 { None } else { Some(rect) }
}
