//! Small utility crate for capturing and encoding screen / window pixels.
//!
//! The base64 helpers come in two flavours:
//!
//! - [`rgba_to_png_base64_raw`] / [`rgb_to_jpeg_base64_raw`] return bare
//!   base64 strings, suitable for binary-safe transport (LLM `ImageContentBlock`,
//!   IPC payloads, etc.) where the receiver knows the mime type out of band.
//! - [`rgba_to_base64`] / [`rgb_to_base64`] wrap the raw helpers in a
//!   `data:<mime>;base64,...` URL, suitable for direct embedding in HTML
//!   `<img src="...">` or CSS.
//!
//! Window/screen capture lives in the [`capture`] submodule, which wraps
//! `xcap` behind an async API and exposes a one-shot
//! [`capture::prime_capture_permission`] hook so the macOS Screen Recording
//! TCC prompt can be triggered at app start instead of on first use.

use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Rgb, Rgba};

pub mod capture;

/// PNG-encode an RGBA image and return the raw base64 payload (no `data:` prefix).
pub fn rgba_to_png_base64_raw(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<String> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);

    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

    Ok(general_purpose::STANDARD.encode(&buffer))
}

/// JPEG-encode an RGB image and return the raw base64 payload (no `data:` prefix).
pub fn rgb_to_jpeg_base64_raw(image: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<String> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);

    image
        .write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

    Ok(general_purpose::STANDARD.encode(&buffer))
}

pub fn rgb_to_base64(image: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<String> {
    let base64 = rgb_to_jpeg_base64_raw(&image)?;
    Ok(format!("data:image/jpeg;base64,{}", base64))
}

pub fn rgba_to_base64(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<String> {
    let base64 = rgba_to_png_base64_raw(image)?;
    Ok(format!("data:image/png;base64,{}", base64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use image::ImageReader;
    use std::io::Cursor;

    fn sample_rgba() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        ImageBuffer::from_fn(4, 4, |x, y| {
            Rgba([(x * 50) as u8, (y * 50) as u8, ((x + y) * 30) as u8, 255])
        })
    }

    #[test]
    fn raw_base64_round_trips_through_png() {
        let original = sample_rgba();
        let encoded = rgba_to_png_base64_raw(&original).expect("encode");
        let bytes = BASE64_STANDARD.decode(&encoded).expect("decode");
        let decoded = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .expect("guess")
            .decode()
            .expect("decode png")
            .to_rgba8();
        assert_eq!(decoded.dimensions(), original.dimensions());
        assert_eq!(decoded.as_raw(), original.as_raw());
    }

    #[test]
    fn data_url_helper_wraps_raw_base64() {
        let original = sample_rgba();
        let raw = rgba_to_png_base64_raw(&original).expect("raw");
        let data_url = rgba_to_base64(&original).expect("data url");
        let expected = format!("data:image/png;base64,{}", raw);
        assert_eq!(data_url, expected);
    }
}
