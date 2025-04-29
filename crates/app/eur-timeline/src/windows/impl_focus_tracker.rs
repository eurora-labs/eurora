use crate::FocusEvent;
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use std::time::Duration;
use windows_sys::Win32::Foundation::{HBITMAP, HDC};
use windows_sys::Win32::Graphics::Gdi::{
    BITMAP, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    GetObject, SRCCOPY, SelectObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetIconInfo;
use windows_sys::Win32::UI::WindowsAndMessaging::ICONINFO;

use super::utils;

#[derive(Debug, Clone)]
pub(crate) struct ImplFocusTracker {}

impl ImplFocusTracker {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl ImplFocusTracker {
    pub fn track_focus<F>(&self, mut on_focus: F) -> anyhow::Result<()>
    where
        F: FnMut(crate::FocusEvent) -> anyhow::Result<()>,
    {
        // Set up the event loop
        let mut last_window = 0;

        loop {
            // Get the current foreground window
            if let Some(hwnd) = utils::get_foreground_window() {
                // Only process if the window has changed
                if hwnd != last_window {
                    last_window = hwnd;

                    // Get window title
                    let title =
                        utils::get_window_title(hwnd).unwrap_or_else(|| String::from("Unknown"));

                    // Get process ID and name
                    let process = match utils::get_window_process_id(hwnd) {
                        Some(pid) => {
                            utils::get_process_name(pid).unwrap_or_else(|| format!("pid-{}", pid))
                        }
                        None => String::from("Unknown"),
                    };

                    // Try to get the window icon
                    let icon_base64 = get_window_icon(hwnd).unwrap_or_default();

                    // Create and send the focus event
                    on_focus(FocusEvent {
                        process,
                        title,
                        icon_base64,
                    })?;
                }
            }

            // Sleep to avoid high CPU usage
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

/* ------------------------------------------------------------ */
/* Helper functions                                              */
/* ------------------------------------------------------------ */

/// Get the window icon as a base64 encoded PNG
fn get_window_icon(hwnd: windows_sys::Win32::Foundation::HWND) -> Result<String> {
    // This is a simplified implementation
    // In a real implementation, we would use more sophisticated methods to extract the icon

    // Try to get the window icon using WM_GETICON
    let icon_handle = unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd,
            windows_sys::Win32::UI::WindowsAndMessaging::WM_GETICON,
            windows_sys::Win32::UI::WindowsAndMessaging::ICON_BIG as usize,
            0,
        ) as windows_sys::Win32::UI::WindowsAndMessaging::HICON
    };

    if icon_handle == 0 {
        // If that fails, try to get the class icon
        let icon_handle = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetClassLongPtrW(
                hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::GCLP_HICON,
            ) as windows_sys::Win32::UI::WindowsAndMessaging::HICON
        };

        if icon_handle == 0 {
            return Err(anyhow::anyhow!("Failed to get window icon"));
        }

        return icon_to_base64(icon_handle);
    }

    icon_to_base64(icon_handle)
}

/// Convert an icon handle to a base64 encoded PNG
fn icon_to_base64(
    icon_handle: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
) -> Result<String> {
    // Get icon info
    let mut icon_info = ICONINFO {
        fIcon: 0,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: 0,
        hbmColor: 0,
    };

    let result = unsafe { GetIconInfo(icon_handle, &mut icon_info) };
    if result == 0 {
        return Err(anyhow::anyhow!("Failed to get icon info"));
    }

    // Get bitmap info
    let mut bitmap = BITMAP {
        bmType: 0,
        bmWidth: 0,
        bmHeight: 0,
        bmWidthBytes: 0,
        bmPlanes: 0,
        bmBitsPixel: 0,
        bmBits: std::ptr::null_mut(),
    };

    let result = unsafe {
        GetObject(
            icon_info.hbmColor as _,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bitmap as *mut _ as _,
        )
    };
    if result == 0 {
        // Clean up resources
        unsafe {
            DeleteObject(icon_info.hbmMask as _);
            DeleteObject(icon_info.hbmColor as _);
        }
        return Err(anyhow::anyhow!("Failed to get bitmap info"));
    }

    let width = bitmap.bmWidth;
    let height = bitmap.bmHeight;

    // Create device contexts and compatible bitmap
    let screen_dc = unsafe { GetDC(0) };
    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    let compatible_bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };

    // Select the bitmap into the memory DC
    let old_bitmap = unsafe { SelectObject(memory_dc, compatible_bitmap as _) };

    // Copy the icon bitmap to our compatible bitmap
    unsafe {
        BitBlt(memory_dc, 0, 0, width, height, screen_dc, 0, 0, SRCCOPY);
    }

    // Create an image buffer to hold the bitmap data
    let mut img_buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width as u32, height as u32);

    // Read the bitmap data pixel by pixel
    for y in 0..height {
        for x in 0..width {
            let pixel = unsafe { windows_sys::Win32::Graphics::Gdi::GetPixel(memory_dc, x, y) };

            let r = ((pixel >> 0) & 0xFF) as u8;
            let g = ((pixel >> 8) & 0xFF) as u8;
            let b = ((pixel >> 16) & 0xFF) as u8;
            let a = 255; // Assume fully opaque

            img_buffer.put_pixel(x as u32, y as u32, Rgba([r, g, b, a]));
        }
    }

    // Clean up resources
    unsafe {
        SelectObject(memory_dc, old_bitmap);
        DeleteObject(compatible_bitmap as _);
        DeleteDC(memory_dc);
        DeleteObject(icon_info.hbmMask as _);
        DeleteObject(icon_info.hbmColor as _);
    }

    // Encode the image as PNG in memory
    let mut png_data = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png_data);
        img_buffer
            .write_to(&mut cursor, image::ImageFormat::Png)
            .context("Failed to encode image as PNG")?;
    }

    // Encode the PNG data as base64
    let base64_png = general_purpose::STANDARD.encode(&png_data);

    // Add the data URL prefix
    Ok(format!("data:image/png;base64,{}", base64_png))
}
