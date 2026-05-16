//! Dominant-color extraction shared by the live focus stream and the
//! persisted-activity listing.
//!
//! Both paths feed `color-thief` an [`image::RgbaImage`] — the live
//! stream already has one from `focus-tracker`, the list path obtains
//! one by [`decode_image`]-ing the PNG bytes pulled from object storage.
//! A `None` result simply collapses to the default surface tokens on the
//! frontend, so every fallible step here returns `Option` rather than
//! `Result`.

use color_thief::ColorFormat;
use image::{GenericImageView, RgbaImage};

use crate::procedures::timeline::AccentColor;

/// Number of palette samples color-thief produces. We only ever read the
/// first entry; 2 is the documented minimum that yields a single
/// dominant colour without paying for a fuller histogram.
const PALETTE_MAX_COLORS: u8 = 2;

/// Quality knob fed to color-thief (1 = highest fidelity, slower). App
/// icons are tiny so the fastest setting still produces stable results.
const PALETTE_QUALITY: u8 = 1;

/// Classify the dominant colour of an already-decoded RGBA image.
///
/// `color-thief` panics on an empty pixel buffer, so we short-circuit
/// 0×0 inputs up front — the live stream sources its icons from
/// `focus-tracker` and should never produce one, but defending here
/// keeps every call site safe.
pub fn accent_from_image(image: &RgbaImage) -> Option<AccentColor> {
    if image.dimensions() == (0, 0) {
        return None;
    }
    color_thief::get_palette(
        image.as_raw(),
        ColorFormat::Rgba,
        PALETTE_QUALITY,
        PALETTE_MAX_COLORS,
    )
    .ok()
    .and_then(|palette| palette.into_iter().next())
    .map(|c| AccentColor::from_rgb(c.r, c.g, c.b))
}

/// Decode `bytes` (PNG, or any format the `image` crate recognises) into
/// an [`RgbaImage`]. Returns `None` on decode failure or a 0×0 image so
/// callers don't have to discriminate between "no accent available" and
/// "icon is malformed" — both render identically in the UI.
pub fn decode_image(bytes: &[u8]) -> Option<RgbaImage> {
    let img = image::load_from_memory(bytes).ok()?;
    if img.dimensions() == (0, 0) {
        return None;
    }
    Some(img.to_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::io::Cursor;

    fn solid_rgba_image(r: u8, g: u8, b: u8) -> RgbaImage {
        RgbaImage::from_fn(8, 8, |_, _| Rgba([r, g, b, 255]))
    }

    fn png_bytes(image: &RgbaImage) -> Vec<u8> {
        let mut buf = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode png");
        buf
    }

    #[test]
    fn accent_from_image_returns_dominant_colour() {
        let image = solid_rgba_image(200, 80, 40);
        let accent = accent_from_image(&image).expect("accent");
        // color-thief quantises slightly; assert the hex is well-formed
        // rather than an exact match.
        assert!(accent.hex.starts_with('#'));
        assert_eq!(accent.hex.len(), 7);
    }

    #[test]
    fn accent_round_trips_through_decode() {
        let image = solid_rgba_image(30, 110, 220);
        let bytes = png_bytes(&image);
        let decoded = decode_image(&bytes).expect("decode");
        let accent = accent_from_image(&decoded).expect("accent");
        // Blue should land in the dark-luminance band → white on-text.
        assert_eq!(accent.on_hex, "#ffffff");
    }

    #[test]
    fn decode_image_returns_none_on_garbage() {
        assert!(decode_image(b"not an image").is_none());
    }

    #[test]
    fn accent_from_zero_sized_image_returns_none() {
        let empty = RgbaImage::new(0, 0);
        assert!(accent_from_image(&empty).is_none());
    }
}
