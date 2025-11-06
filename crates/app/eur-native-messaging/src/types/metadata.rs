use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeMetadata {}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeIcon {
    pub base64: Option<String>,
}
