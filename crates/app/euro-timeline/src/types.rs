use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub name: String,
    pub icon: Option<Arc<image::RgbaImage>>,
}
