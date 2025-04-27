use anyhow::{Result, anyhow};
// use image::{ImageBuffer, Rgb, Rgba};
use image::{ColorType, ImageBuffer, Rgb, Rgba, codecs::jpeg::JpegEncoder};
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

/// Captures a specific region of the screen
///
/// # Arguments
///
/// * `monitor` - The monitor to capture from
/// * `x` - The x coordinate of the top-left corner of the region
/// * `y` - The y coordinate of the top-left corner of the region
/// * `width` - The width of the region
/// * `height` - The height of the region
///
/// # Returns
///
/// An ImageBuffer containing the captured region
pub fn capture_monitor_region(
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
        .map_err(|e| anyhow!("Failed to capture region: {}", e))?;

    Ok(image_region)
}

/// Captures a region at the specified position with the given dimensions
///
/// # Arguments
///
/// * `x` - The x coordinate of the top-left corner of the region
/// * `y` - The y coordinate of the top-left corner of the region
/// * `width` - The width of the region
/// * `height` - The height of the region
///
/// # Returns
///
/// An ImageBuffer containing the captured region
pub fn capture_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    // Get the primary monitor
    let monitor = Monitor::all()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No monitors found"))?;

    // Ensure x and y are positive
    let x = if x < 0 { 0 } else { x as u32 };
    let y = if y < 0 { 0 } else { y as u32 };

    capture_monitor_region(monitor, x, y, width, height)
}

/// Converts an ImageBuffer to a base64 encoded PNG string
///
/// # Arguments
///
/// * `image` - The ImageBuffer to convert
///
/// # Returns
///
/// A base64 encoded PNG string  
pub fn image_to_base64(image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<String> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);

    let rgb = rgba_to_rgb(image);

    rgb.write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

    let base64 = base64::encode(&buffer);
    Ok(format!("data:image/jpeg;base64,{}", base64))
}
fn rgba_to_rgb(rgba_img: ImageBuffer<Rgba<u8>, Vec<u8>>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let (width, height) = rgba_img.dimensions();
    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);

    for pixel in rgba_img.pixels() {
        rgb_data.extend_from_slice(&pixel.0[..3]); // Take only R, G, B
    }

    ImageBuffer::from_raw(width, height, rgb_data).expect("Failed to create RGB image")
}
