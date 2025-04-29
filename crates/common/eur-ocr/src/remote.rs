use crate::OcrStrategy;

pub struct RemoteOcr {}

impl OcrStrategy for RemoteOcr {
    fn recognize(&self, image: &image::DynamicImage) -> String {
        todo!()
    }
}
