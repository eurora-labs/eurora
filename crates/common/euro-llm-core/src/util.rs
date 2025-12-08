#[cfg(feature = "dynamic-image")]
pub mod dynamic_image {
    use base64::{Engine, engine::general_purpose::STANDARD as B64};
    use image::{DynamicImage, ImageFormat};
    use std::io::Cursor;

    pub fn image_to_base64(img: &DynamicImage) -> String {
        // 1. Encode image to bytes once.
        let mut bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("image encoding failed");
        const PREFIX: &str = "data:image/png;base64,";
        // Pre-size: base64 length = 4 * ceil(n / 3)
        let b64_len = bytes.len().div_ceil(3) * 4;
        let mut out = String::with_capacity(PREFIX.len() + b64_len);
        out.push_str(PREFIX);
        B64.encode_string(&bytes, &mut out);
        out
    }
}
