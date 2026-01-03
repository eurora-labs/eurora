pub use crate::proto::Activity;
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use focus_tracker_core::FocusedWindow;
use prost_types::Timestamp;
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

impl Activity {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7().to_string(),
            name,
            process_name: None,
            window_title: None,
            icon: None,
            started_at: Some(Timestamp {
                seconds: now.timestamp(),
                nanos: now.timestamp_subsec_nanos() as i32,
            }),
            ended_at: None,
            created_at: None,
            updated_at: None,
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
