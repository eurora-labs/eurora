use crate::OcrStrategy;

pub struct RemoteOcr {}

impl OcrStrategy for RemoteOcr {
    fn recognize(&self, _image: &image::DynamicImage) -> String {
        todo!()
    }
}
