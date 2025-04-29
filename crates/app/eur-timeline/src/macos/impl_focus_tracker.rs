use crate::FocusEvent;
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use std::process::Command;
use std::time::Duration;

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
        loop {
            // Get the current focused window information
            match get_focused_window_info() {
                Ok((process, title, icon_data)) => {
                    // Convert icon data to base64 if available
                    let icon_base64 = icon_data
                        .map(|data| convert_icon_to_base64(&data))
                        .unwrap_or_else(|| Ok(String::new()))
                        .unwrap_or_default();

                    // Create and send the focus event
                    on_focus(FocusEvent {
                        process,
                        title,
                        icon_base64,
                    })?;
                }
                Err(e) => {
                    eprintln!("Error getting window info: {}", e);
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

/// Get information about the currently focused window
fn get_focused_window_info() -> Result<(String, String, Option<Vec<u32>>)> {
    // Get the frontmost application name using AppleScript
    let process = utils::get_frontmost_app_name()
        .ok_or_else(|| anyhow::anyhow!("Failed to get frontmost application name"))?;

    // Get the frontmost window title
    let title = utils::get_frontmost_window_title()
        .unwrap_or_else(|| format!("{} (No window title)", process));

    // Try to get the application icon
    let icon_data = get_app_icon(&process).ok();

    Ok((process, title, icon_data))
}

/// Get the application icon for a given process name
fn get_app_icon(process_name: &str) -> Result<Vec<u32>> {
    // This is a simplified implementation using AppleScript to get the app icon
    // In a real implementation, we would use NSImage and other Cocoa APIs

    // Create a temporary file to save the icon
    let temp_file = format!("/tmp/app_icon_{}.png", std::process::id());

    // AppleScript to extract the application icon and save it to a file
    let script = format!(
        r#"
        try
            tell application "Finder"
                set appPath to application file "{}" as alias
                set appIcon to icon of appPath
                set tempFolder to path to temporary items as string
                set tempFile to "{}"
                
                tell application "System Events"
                    set iconFile to (make new file at tempFolder with properties {{name:tempFile}})
                    set iconPath to path of iconFile
                end tell
                
                copy appIcon to iconFile
                return POSIX path of iconPath
            end tell
        on error
            return ""
        end try
        "#,
        process_name, temp_file
    );

    // Execute the AppleScript
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .context("Failed to execute AppleScript")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("AppleScript execution failed"));
    }

    // Check if the icon file was created
    let icon_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if icon_path.is_empty() {
        return Err(anyhow::anyhow!("Failed to get application icon"));
    }

    // Load the icon image
    let img = image::open(&icon_path)
        .context("Failed to open icon image")?
        .to_rgba8();

    // Convert the image to the expected format (width, height, ARGB pixels)
    let width = img.width();
    let height = img.height();

    let mut icon_data = Vec::with_capacity(2 + (width * height) as usize);
    icon_data.push(width as u32);
    icon_data.push(height as u32);

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let argb = ((pixel[3] as u32) << 24)
                | ((pixel[0] as u32) << 16)
                | ((pixel[1] as u32) << 8)
                | (pixel[2] as u32);
            icon_data.push(argb);
        }
    }

    // Clean up the temporary file
    let _ = std::fs::remove_file(icon_path);

    Ok(icon_data)
}

/// Convert ARGB icon data to a base64 encoded PNG image
fn convert_icon_to_base64(icon_data: &[u32]) -> Result<String> {
    if icon_data.len() < 2 {
        return Err(anyhow::anyhow!("Invalid icon data"));
    }

    let width = icon_data[0] as u32;
    let height = icon_data[1] as u32;

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
