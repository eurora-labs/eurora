use anyhow::{Result, anyhow};
use image::{ImageBuffer, Rgba};
use xcap::Monitor;

/// Captures the entire primary monitor and returns an ImageBuffer
pub fn capture_monitor() -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    // Get the primary monitor (first one)
    let monitor = Monitor::all()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No monitors found"))?;

    // Capture the entire monitor
    let image = monitor.capture_image()?;

    // Convert to an image::ImageBuffer
    let width = image.width() as u32;
    let height = image.height() as u32;
    let raw_data = image.into_raw();

    // Create an ImageBuffer from the raw data
    let img_buffer = ImageBuffer::from_raw(width, height, raw_data)
        .ok_or_else(|| anyhow!("Failed to create image buffer"))?;

    Ok(img_buffer)
}

/// Captures all available monitors and returns a vector of ImageBuffer for each monitor
pub fn capture_all_monitors() -> Result<Vec<ImageBuffer<Rgba<u8>, Vec<u8>>>> {
    let monitors = Monitor::all()?;
    let mut images = Vec::new();

    for monitor in monitors {
        let image = monitor.capture_image()?;
        let width = image.width() as u32;
        let height = image.height() as u32;
        let raw_data = image.into_raw();

        let img_buffer = ImageBuffer::from_raw(width, height, raw_data)
            .ok_or_else(|| anyhow!("Failed to create image buffer"))?;

        images.push(img_buffer);
    }

    Ok(images)
}

pub fn capture_region(
    monitor: Monitor,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let monitor_width = monitor.width().unwrap();
    let monitor_height = monitor.height().unwrap();

    let region_width = width.min(monitor_width - x) as u32;
    let region_height = height.min(monitor_height - y) as u32;

    let image_region = monitor
        .capture_region(x as i32, y as i32, region_width, region_height)
        .unwrap();

    Ok(image_region)
}
