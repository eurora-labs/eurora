use crate::FocusEvent;
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use std::time::Duration;
use windows_sys::Win32::Foundation::HWND;

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
        // Track the previously focused window to avoid duplicate events
        let mut prev_hwnd: Option<HWND> = None;
        let mut prev_title: Option<String> = None;

        // Get initial focused window
        if let Some(hwnd) = utils::get_foreground_window() {
            if let Ok((title, process)) = utils::get_window_info(hwnd) {
                let icon_base64 = get_window_icon(hwnd).unwrap_or_default();
                
                if let Err(e) = on_focus(FocusEvent {
                    process,
                    title: title.clone(),
                    icon_base64,
                }) {
                    eprintln!("Focus event handler failed: {}", e);
                }
                
                prev_hwnd = Some(hwnd);
                prev_title = Some(title);
            }
        }

        // Main event loop - we'll use polling since Windows event hooks are complex to integrate
        // with Rust's async runtime in a cross-platform way
        loop {
            // Check current foreground window
            if let Some(current_hwnd) = utils::get_foreground_window() {
                let focus_changed = match prev_hwnd {
                    Some(prev) => prev != current_hwnd,
                    None => true,
                };

                match utils::get_window_info(current_hwnd) {
                    Ok((title, process)) => {
                        // Also check if title changed for the same window
                        let title_changed = match &prev_title {
                            Some(prev_t) => *prev_t != title,
                            None => true,
                        };

                        // Trigger handler if either window focus or title has changed
                        if focus_changed || title_changed {
                            let icon_base64 = get_window_icon(current_hwnd).unwrap_or_default();
                            
                            if let Err(e) = on_focus(FocusEvent {
                                process,
                                title: title.clone(),
                                icon_base64,
                            }) {
                                eprintln!("Focus event handler failed: {}", e);
                            }
                            
                            prev_hwnd = Some(current_hwnd);
                            prev_title = Some(title);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get window info: {}", e);
                    }
                }
            } else {
                // No foreground window
                if prev_hwnd.is_some() {
                    prev_hwnd = None;
                    prev_title = None;
                }
            }

            // Sleep to avoid high CPU usage
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

/* ------------------------------------------------------------ */
/* Helper functions                                              */
/* ------------------------------------------------------------ */

/// Get the icon for a window (simplified implementation)
fn get_window_icon(_hwnd: HWND) -> Result<String> {
    // For now, return empty string as getting window icons on Windows
    // requires more complex Win32 API calls and icon extraction
    // This could be enhanced later with proper icon extraction
    
    // We would need to:
    // 1. Get the window's class icon or application icon
    // 2. Extract the icon data
    // 3. Convert to PNG format
    // 4. Encode as base64
    
    // For the initial implementation, we'll return empty string
    // which matches the behavior when icon extraction fails in other platforms
    Ok(String::new())
}

/// Convert icon data to base64 PNG (placeholder for future implementation)
fn _convert_icon_to_base64(icon_data: &[u32]) -> Result<String> {
    if icon_data.len() < 2 {
        return Err(anyhow::anyhow!("Invalid icon data"));
    }

    let width = icon_data[0];
    let height = icon_data[1];

    if width == 0 || height == 0 || width > 1024 || height > 1024 {
        return Err(anyhow::anyhow!("Invalid icon dimensions"));
    }

    // Create an image buffer
    let mut img = ImageBuffer::new(width, height);

    // Fill the image with the icon data
    for y in 0..height {
        for x in 0..width {
            let idx = 2 + (y * width + x) as usize;
            if idx < icon_data.len() {
                let argb = icon_data[idx];
                let a = ((argb >> 24) & 0xFF) as u8;
                let r = ((argb >> 16) & 0xFF) as u8;
                let g = ((argb >> 8) & 0xFF) as u8;
                let b = (argb & 0xFF) as u8;
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
    }

    // Encode the image as PNG in memory
    let mut png_data = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png_data);
        img.write_to(&mut cursor, image::ImageFormat::Png)
            .context("Failed to encode image as PNG")?;
    }

    // Encode the PNG data as base64
    let base64_png = general_purpose::STANDARD.encode(&png_data);

    // Add the data URL prefix
    Ok(format!("data:image/png;base64,{}", base64_png))
}