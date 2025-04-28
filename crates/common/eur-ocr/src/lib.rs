pub mod tesseract;
pub use tesseract::TesseractOcr;

pub mod remote;

use image::DynamicImage;

pub trait OcrStrategy {
    fn recognize(&self, image: &DynamicImage) -> String;
}
