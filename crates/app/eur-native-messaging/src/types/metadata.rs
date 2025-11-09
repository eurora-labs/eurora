use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeMetadata {
    pub url: Option<String>,
    pub icon_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeIcon {
    pub base64: Option<String>,
}
