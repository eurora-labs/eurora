use anyhow::{Result, anyhow};
// use image::{ImageBuffer, Rgb, Rgba};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgb, Rgba};
use xcap::Monitor;

// use eur_ocr::{self};

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
    let width = image.width();
    let height = image.height();
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
        let width = image.width();
        let height = image.height();
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
    // let monitor_width = monitor.width().unwrap();
    // let monitor_height = monitor.height().unwrap();

    // let region_width = width.min(monitor_width - x) as u32;
    // let region_height = height.min(monitor_height - y) as u32;

    let image_region = monitor
        .capture_region(x as i32, y as i32, width, height)
        .map_err(|e| anyhow!("Failed to capture region: {}", e))?;

    Ok(image_region)
}

pub fn capture_monitor_region_rgba(
    monitor: Monitor,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    // let monitor_width = monitor.width().unwrap();
    // let monitor_height = monitor.height().unwrap();

    // let region_width = width.min(monitor_width - x) as u32;
    // let region_height = height.min(monitor_height - y) as u32;

    let image_region = monitor
        .capture_region(x as i32, y as i32, width, height)
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

pub fn capture_focused_region_rgba(
    monitor_name: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let monitor = Monitor::all()?
        .into_iter()
        .find(|monitor| monitor.name().unwrap() == monitor_name)
        .ok_or_else(|| anyhow!("No monitors found"))?;
    capture_monitor_region_rgba(monitor, x, y, width, height)
}

pub fn capture_region_rgba(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    eprintln!(
        "capture_region_rgba: x: {}, y: {}, width: {}, height: {}",
        x, y, width, height
    );
    // Get the primary monitor
    let monitor = Monitor::all()?
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| anyhow!("No monitors found"))?;

    // Ensure x and y are positive
    let x = if x < 0 { 0 } else { x as u32 };
    let y = if y < 0 { 0 } else { y as u32 };

    let image = capture_monitor_region_rgba(monitor, x, y, width, height)?;

    // // TODO: remove this code
    // let tess = eur_ocr::TesseractOcr {};
    // let result_text = tess.recognize(&image::DynamicImage::ImageRgb8(image.clone()));
    // eprintln!("Recognized text: {}", result_text);

    Ok(image)
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
pub fn image_to_base64(image: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<String> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);

    // let start = std::time::Instant::now();
    // let rgb = image::DynamicImage::ImageRgba8(image).to_rgb8();
    // // let rgb = rgba_to_rgb(image);
    // let duration = start.elapsed();
    // eprintln!("Conversion to RGB completed in: {:?}", duration);

    image
        .write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

    let base64 = general_purpose::STANDARD.encode(&buffer);
    // let base64 = base64::encode(&buffer);
    Ok(format!("data:image/jpeg;base64,{}", base64))
}

// pub fn image_to_base64(image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<String> {
//     let mut buffer = Vec::new();
//     let mut cursor = std::io::Cursor::new(&mut buffer);

//     let start = std::time::Instant::now();
//     let rgb = image::DynamicImage::ImageRgba8(image).to_rgb8();
//     // let rgb = rgba_to_rgb(image);
//     let duration = start.elapsed();
//     eprintln!("Conversion to RGB completed in: {:?}", duration);

//     rgb.write_to(&mut cursor, image::ImageFormat::Jpeg)
//         .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

//     let base64 = base64::encode(&buffer);
//     Ok(format!("data:image/jpeg;base64,{}", base64))
// }
