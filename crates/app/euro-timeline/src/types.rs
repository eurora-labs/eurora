/// Event emitted when focus changes to a new application
#[derive(Debug, Clone)]
pub struct ActivityEvent {
    /// The name of the activity
    pub name: String,
    /// The icon of the application (if available)
    pub icon: Option<image::RgbaImage>,
}
