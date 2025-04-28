use anyhow::{Result, anyhow};
use image::{ImageBuffer, Rgba};
use std::time::Instant;
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

/// Captures the entire primary monitor and measures the time it took
pub fn capture_monitor_timed() -> Result<(ImageBuffer<Rgba<u8>, Vec<u8>>, std::time::Duration)> {
    let start = Instant::now();
    let image = capture_monitor()?;
    let duration = start.elapsed();

    Ok((image, duration))
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

/// Captures a specific region of the primary monitor
/// Captures a 600x500 region starting from 1/4th of the monitor width
pub fn capture_region() -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    // Get the primary monitor (first one)
    let monitor = Monitor::all()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No monitors found"))?;

    // Capture the entire monitor first
    // let full_image = monitor.capture_image()?;

    let width = monitor.width().unwrap() as i32;
    let height = monitor.height().unwrap() as i32;
    let start_x = width / 4; // Start from 1/4th of monitor width
    let image_region = monitor
        .capture_region(
            start_x,
            0,
            1100.min((width - start_x) as u32),
            150.min(height as u32),
        )
        .unwrap();

    Ok(image_region)

    // // Convert to an image::ImageBuffer
    // let width = full_image.width() as u32;
    // let height = full_image.height() as u32;
    // let raw_data = full_image.into_raw();

    // // Create an ImageBuffer from the raw data
    // let full_img_buffer = ImageBuffer::from_raw(width, height, raw_data)
    //     .ok_or_else(|| anyhow!("Failed to create image buffer"))?;

    // // Calculate the region to capture
    // let start_x = width / 4; // Start from 1/4th of monitor width
    // let crop_width = 600.min(width - start_x); // Ensure we don't go beyond the image boundary
    // let crop_height = 500.min(height); // Ensure we don't go beyond the image boundary

    // // Crop the image to the desired region
    // let cropped_img =
    //     image::imageops::crop_imm(&full_img_buffer, start_x, 0, crop_width, crop_height).to_image();

    // Ok(cropped_img)
}
