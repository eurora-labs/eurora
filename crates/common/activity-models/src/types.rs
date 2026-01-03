pub use crate::proto::Activity;
use specta::Type;
use std::collections::HashMap;
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
    // /// Create a new activity
    // pub fn new(
    //     name: String,
    //     icon: Option<String>,
    //     name: String,
    //     assets: Vec<ActivityAsset>,
    // ) -> Self {
    //     Self {
    //         id: Uuid::new_v4().to_string(),
    //         name,
    //         icon,
    //         process_name,
    //         start: Utc::now(),
    //         end: None,
    //         assets,
    //         snapshots: Vec::new(),
    //     }
    // }
    /// Get context chips for UI integration
    pub fn get_attachment_chips(&self) -> Vec<AttachmentChip> {
        todo!()
    }
}
