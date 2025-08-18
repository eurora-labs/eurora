use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct HoverSettings {
    /// Whether hover window is enabled
    pub enabled: bool,
    // /// Position of hover window
    // pub position: (i64, i64),
}
