use euro_activity::ContextChip;
use tauri::Runtime;

#[taurpc::ipc_type]
pub struct AccentColor {
    /// Dominant color in CSS form: lowercase `#rrggbb`.
    pub hex: String,
    /// Text/foreground color (`#000000` or `#ffffff`) chosen via WCAG relative
    /// luminance. Use for text rendered on top of `hex`.
    pub on_hex: String,
    /// Icon-background color (`#000000` or `#ffffff`) chosen via NTSC
    /// perceived brightness. Use for shapes that visually contrast with `hex`.
    pub icon_bg: String,
}

impl AccentColor {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let on_hex = pick_contrast(relative_luminance(r, g, b));
        let icon_bg = pick_contrast(perceived_brightness(r, g, b));
        Self {
            hex: format!("#{r:02x}{g:02x}{b:02x}"),
            on_hex: on_hex.to_string(),
            icon_bg: icon_bg.to_string(),
        }
    }
}

fn pick_contrast(value: f64) -> &'static str {
    if value > 0.5 { "#000000" } else { "#ffffff" }
}

fn perceived_brightness(r: u8, g: u8, b: u8) -> f64 {
    (0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) / 255.0
}

fn srgb_to_linear(channel: u8) -> f64 {
    let c = channel as f64 / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b)
}

#[taurpc::ipc_type]
pub struct TimelineAppEvent {
    pub name: String,
    pub accent: Option<AccentColor>,
    pub icon_base64: Option<String>,
}

#[taurpc::procedures(path = "timeline")]
pub trait TimelineApi {
    #[taurpc(event)]
    async fn new_app_event(event: TimelineAppEvent);

    #[taurpc(event)]
    async fn new_assets_event(chips: Vec<ContextChip>);

    async fn list<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<Vec<String>, String>;
}

#[derive(Clone)]
pub struct TimelineApiImpl;

#[taurpc::resolvers]
impl TimelineApi for TimelineApiImpl {
    async fn list<R: Runtime>(
        self,
        _app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
}
