use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgb, Rgba};
use xcap::Monitor;

pub fn capture_monitor() -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let monitor = Monitor::all()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No monitors found"))?;

    let image = monitor.capture_image()?;

    let width = image.width();
    let height = image.height();
    let raw_data = image.into_raw();

    let img_buffer = ImageBuffer::from_raw(width, height, raw_data)
        .ok_or_else(|| anyhow!("Failed to create image buffer"))?;

    Ok(img_buffer)
}

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

pub fn capture_monitor_region(
    monitor: Monitor,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    tracing::debug!("Capturing monitor region");

    let image_region = monitor
        .capture_region(x, y, width, height)
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
    let image_region = monitor
        .capture_region(x, y, width, height)
        .map_err(|e| anyhow!("Failed to capture region: {}", e))?;

    Ok(image_region)
}

pub fn capture_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let monitor = Monitor::all()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No monitors found"))?;

    let x = if x < 0 { 0 } else { x as u32 };
    let y = if y < 0 { 0 } else { y as u32 };

    capture_monitor_region(monitor, x, y, width, height)
}

pub fn get_all_monitors() -> Result<Vec<Monitor>> {
    Ok(Monitor::all()?)
}

pub fn capture_monitor_by_id(monitor_id: &String) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let monitors = Monitor::all()?;
    if monitors.is_empty() {
        return Err(anyhow!("No monitors found"));
    }
    if monitors.len() == 1 {
        return Ok(monitors[0].capture_image()?);
    }
    let requested = monitor_id
        .parse::<u32>()
        .map_err(|_| anyhow!("Invalid monitor id '{}'", monitor_id))?;

    let some_monitor = monitors
        .into_iter()
        .find(|m| m.id().unwrap_or_default() == requested)
        .ok_or_else(|| anyhow!("Monitor '{}' not found", monitor_id))?;

    Ok(some_monitor.capture_image()?)
}

pub fn capture_focused_region_rgba(
    monitor_id: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let monitors = Monitor::all()?;
    if monitors.is_empty() {
        return Err(anyhow!("No monitors found"));
    }
    if monitors.len() == 1 {
        return Ok(monitors[0].capture_region(x, y, width, height)?);
    }
    let requested = monitor_id
        .parse::<u32>()
        .map_err(|_| anyhow!("Invalid monitor id '{}'", monitor_id))?;

    let some_monitor = monitors
        .into_iter()
        .find(|m| m.id().unwrap_or_default() == requested)
        .ok_or_else(|| anyhow!("Monitor '{}' not found", monitor_id))?;
    capture_monitor_region_rgba(some_monitor, x, y, width, height)
}

pub fn capture_region_rgba(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let monitor = Monitor::all()?
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| anyhow!("No monitors found"))?;

    let x = if x < 0 { 0 } else { x as u32 };
    let y = if y < 0 { 0 } else { y as u32 };

    let image = capture_monitor_region_rgba(monitor, x, y, width, height)?;

    Ok(image)
}

pub fn rgb_to_base64(image: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<String> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);

    image
        .write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

    let base64 = general_purpose::STANDARD.encode(&buffer);
    Ok(format!("data:image/jpeg;base64,{}", base64))
}

pub fn rgba_to_base64(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<String> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);

    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

    let base64 = general_purpose::STANDARD.encode(&buffer);
    Ok(format!("data:image/png;base64,{}", base64))
}
