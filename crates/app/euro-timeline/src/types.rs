#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub name: String,
    pub icon: Option<image::RgbaImage>,
}
