pub use crate::proto;
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use focus_tracker_core::FocusedWindow;
use serde::{Deserialize, Serialize};
// use image;
use specta::Type;
use std::collections::HashMap;
use tracing::error;
use uuid::Uuid;

/// Context chip for UI integration
#[derive(Type)]
pub struct AttachmentChip {
    pub id: Uuid,
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub icon: Option<String>,
}

/// Main activity structure - now fully cloneable and serializable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// ID of the activity
    pub id: Uuid,
    /// Name of the activity
    pub name: String,
    /// Icon representing the activity
    pub icon: Option<String>,
    /// Process name of the activity
    pub process_name: Option<String>,
    /// Window title of the activity
    pub window_title: Option<String>,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Assets associated with the activity
    pub assets: Vec<Uuid>,
}

impl Activity {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::now_v7(),
            name,
            process_name: None,
            window_title: None,
            icon: None,
            started_at: Utc::now(),
            ended_at: None,
            assets: Vec::new(),
        }
    }

    /// Get context chips for UI integration
    pub fn get_attachment_chips(&self) -> Vec<AttachmentChip> {
        todo!()
    }

    pub fn with_focus_window(mut self, window: FocusedWindow) -> Self {
        self.icon = match window.icon {
            Some(icon) => {
                let mut buffer = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut buffer);
                match icon.write_to(&mut cursor, image::ImageFormat::Png) {
                    Ok(_) => {
                        let base64 = general_purpose::STANDARD.encode(&buffer);
                        Some(format!("data:image/png;base64,{}", base64))
                    }
                    Err(e) => {
                        error!("Failed to encode image: {}", e);
                        None
                    }
                }
            }
            None => None,
        };
        self.name = window.process_name.clone();
        self.process_name = Some(window.process_name);
        self.window_title = window.window_title;
        self
    }
}
