use image::{ImageBuffer, Rgba};
use resvg::render;
use tiny_skia::Pixmap;
use usvg::{Options, Tree};

use crate::error::{ActivityError, ActivityResult};

pub fn render_svg_bytes(svg_bytes: &[u8]) -> ActivityResult<image::RgbaImage> {
    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = Tree::from_data(svg_bytes, &opt)
        .map_err(|e| ActivityError::invalid_data(format!("Failed to parse SVG: {}", e)))?;

    let size = tree.size();
    let target = 48.0_f32;
    let scale = (target / size.width().max(size.height())).max(1.0);
    let width = (size.width() * scale).ceil() as u32;
    let height = (size.height() * scale).ceil() as u32;

    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| {
        ActivityError::invalid_data(format!(
            "Failed to create pixmap with dimensions {}x{}",
            width, height
        ))
    })?;

    render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixmap.data().to_vec())
        .ok_or_else(|| {
            ActivityError::invalid_data(format!(
                "Failed to create image buffer from pixmap data ({}x{})",
                width, height
            ))
        })?;

    Ok(img)
}
