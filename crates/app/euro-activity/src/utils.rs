use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use image::{ImageBuffer, Rgba};
use resvg::render;
use tiny_skia::Pixmap;
use usvg::{Options, Tree};

use crate::error::{ActivityError, ActivityResult};

pub fn convert_svg_to_rgba(svg: &str) -> ActivityResult<image::RgbaImage> {
    let b64 = svg
        .trim()
        .strip_prefix("data:image/svg+xml;base64,")
        .unwrap_or(svg);

    let svg_bytes = BASE64_STANDARD
        .decode(b64)
        .map_err(|e| ActivityError::invalid_data(format!("Failed to decode base64 SVG: {}", e)))?;

    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = Tree::from_data(&svg_bytes, &opt)
        .map_err(|e| ActivityError::invalid_data(format!("Failed to parse SVG: {}", e)))?;

    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| {
        ActivityError::invalid_data(format!(
            "Failed to create pixmap with dimensions {}x{}",
            width, height
        ))
    })?;

    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixmap.data().to_vec())
        .ok_or_else(|| {
            ActivityError::invalid_data(format!(
                "Failed to create image buffer from pixmap data ({}x{})",
                width, height
            ))
        })?;

    Ok(img)
}
