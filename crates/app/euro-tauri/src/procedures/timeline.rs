use euro_activity::ContextChip;
use euro_timeline::TimelineManager;
use euro_vision::rgba_to_base64;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::sync::Mutex;

use crate::procedures::accent::accent_from_image;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct TimelineAppEvent {
    pub name: String,
    pub accent: Option<AccentColor>,
    pub icon_base64: Option<String>,
    /// Executable name of the focused process. Used by the frontend to
    /// resolve which browser (if any) the user is currently on so it can
    /// surface the matching extension install affordance.
    pub process_name: String,
    /// OS-level process id of the focused process. Required when the
    /// frontend wants to act on that specific browser instance (for
    /// example, opening a URL inside it rather than the OS default).
    pub process_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct TimelineAssetsEvent(pub Vec<ContextChip>);

/// Returns the currently-focused activity in the same shape the
/// [`TimelineAppEvent`] broadcast uses. The broadcast channel does
/// not replay history, so a webview that mounts mid-session (the
/// ask / answer overlay windows are the load-bearing example) would
/// otherwise see no icon until the next focus change. The overlay
/// pages call this on mount to seed the icon synchronously instead
/// of waiting on the user to switch apps.
///
/// Returns `None` when the timeline collector hasn't observed any
/// activity yet (cold startup, before the first focus event).
#[tauri::command]
#[specta::specta]
pub async fn timeline_get_current_app(app_handle: AppHandle) -> Option<TimelineAppEvent> {
    let timeline_state = app_handle.try_state::<Mutex<TimelineManager>>()?;
    let timeline = timeline_state.lock().await;
    let storage = timeline.storage.lock().await;
    let activity = storage.get_current_activity()?;

    let (accent, icon_base64) = match activity.icon.as_ref() {
        Some(icon) => (accent_from_image(icon), rgba_to_base64(icon).ok()),
        None => (None, None),
    };

    Some(TimelineAppEvent {
        name: activity.name.clone(),
        accent,
        icon_base64,
        process_name: activity.process_name.clone(),
        process_id: activity.process_id,
    })
}
