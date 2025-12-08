use std::collections::HashMap;

use rusty_tesseract::Args;

use crate::OcrStrategy;

#[derive(Default)]
pub struct TesseractOcr {}

impl OcrStrategy for TesseractOcr {
    fn recognize(&self, image: &image::DynamicImage) -> String {
        let args = Args {
            lang: "eng".into(),
            config_variables: HashMap::from([("tessedit_create_tsv".into(), "1".into())]),
            dpi: Some(600),
            psm: Some(1),
            oem: Some(1),
        };
        let tess_image = rusty_tesseract::Image::from_dynamic_image(image).unwrap();
        let data = rusty_tesseract::image_to_data(&tess_image, &args).unwrap();

        let mut text = String::new();
        for record in &data.data {
            if record.text.is_empty() {
                continue;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&record.text);
        }

        text
    }
}
