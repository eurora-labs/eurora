use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FocusedWindow {
    pub process_id: u32,
    pub process_name: String,
    pub window_title: Option<String>,
    pub icon: Option<Arc<image::RgbaImage>>,
}
