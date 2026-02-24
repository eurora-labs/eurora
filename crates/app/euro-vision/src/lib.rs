use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgb, Rgba};

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
