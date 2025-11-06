use cfg_if::cfg_if;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use image::ImageBuffer;
use image::Rgba;
use resvg::render;
use tiny_skia::Pixmap;
use usvg::{Options, Tree};

#[inline(always)]
pub fn os_pick<'a>(_windows: &'a str, _linux: &'a str, _macos: &'a str) -> &'a str {
    cfg_if! {
        if #[cfg(target_os = "windows")] { _windows }
        else if #[cfg(target_os = "linux")] { _linux }
        else if #[cfg(target_os = "macos")] { _macos }
        else { compile_error!("Unsupported target OS"); }
    }
}

pub fn convert_svg_to_rgba(svg: &str) -> Option<image::RgbaImage> {
    let b64 = svg
        .trim()
        .strip_prefix("data:image/svg+xml;base64,")
        .unwrap_or(&svg);
    let svg_bytes = BASE64_STANDARD.decode(b64).ok();
    if svg_bytes.is_none() {
        return None;
    }
    let svg_bytes = svg_bytes.unwrap();
    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = Tree::from_data(&svg_bytes, &opt).unwrap();
    let mut pixmap = Pixmap::new(
        opt.default_size.width() as u32,
        opt.default_size.height() as u32,
    )
    .unwrap();
    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(100, 100, pixmap.data().to_vec())
        .ok_or("Failed to create image buffer")
        .unwrap();
    Some(img)
}
