use cfg_if::cfg_if;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use image::{ImageBuffer, Rgba};
use resvg::render;
use tiny_skia::Pixmap;
use usvg::{Options, Tree};

use crate::error::{ActivityError, ActivityResult};

#[inline(always)]
pub fn os_pick<'a>(_windows: &'a str, _linux: &'a str, _macos: &'a str) -> &'a str {
    cfg_if! {
        if #[cfg(target_os = "windows")] { _windows }
        else if #[cfg(target_os = "linux")] { _linux }
        else if #[cfg(target_os = "macos")] { _macos }
        else { compile_error!("Unsupported target OS"); }
    }
}

/// Converts an SVG string (either base64-encoded or raw) to an RGBA image.
///
/// # Arguments
///
/// * `svg` - SVG content as either a data URL (`data:image/svg+xml;base64,...`) or base64 string
///
/// # Returns
///
/// Returns an `RgbaImage` on success, or an `ActivityError` if:
/// - Base64 decoding fails
/// - SVG parsing fails
/// - Pixmap creation fails
/// - Image buffer creation fails
///
/// # Example
///
/// ```ignore
/// let svg_data = "data:image/svg+xml;base64,PHN2Zy...";
/// let image = convert_svg_to_rgba(svg_data)?;
/// ```
pub fn convert_svg_to_rgba(svg: &str) -> ActivityResult<image::RgbaImage> {
    // Strip data URL prefix if present
    let b64 = svg
        .trim()
        .strip_prefix("data:image/svg+xml;base64,")
        .unwrap_or(svg);

    // Decode base64 SVG data
    let svg_bytes = BASE64_STANDARD
        .decode(b64)
        .map_err(|e| ActivityError::invalid_data(format!("Failed to decode base64 SVG: {}", e)))?;

    // Parse SVG with system fonts loaded
    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = Tree::from_data(&svg_bytes, &opt)
        .map_err(|e| ActivityError::invalid_data(format!("Failed to parse SVG: {}", e)))?;

    // Get actual SVG dimensions
    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    // Create pixmap with correct dimensions
    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| {
        ActivityError::invalid_data(format!(
            "Failed to create pixmap with dimensions {}x{}",
            width, height
        ))
    })?;

    // Render SVG to pixmap
    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // Convert pixmap to image buffer
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixmap.data().to_vec())
        .ok_or_else(|| {
            ActivityError::invalid_data(format!(
                "Failed to create image buffer from pixmap data ({}x{})",
                width, height
            ))
        })?;

    Ok(img)
}
