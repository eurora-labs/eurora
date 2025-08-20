use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct Hotkey {
    pub modifiers: Vec<String>,
    pub key: String,
}

impl Default for Hotkey {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                modifiers: vec!["Command".to_string(), "Shift".to_string()],
                key: "S".to_string(),
            }
        }

        #[cfg(target_os = "linux")]
        {
            Self {
                modifiers: vec!["Super".to_string()],
                key: "Space".to_string(),
            }
        }

        #[cfg(target_os = "windows")]
        {
            Self {
                modifiers: vec!["Ctrl".to_string(), "Shift".to_string()],
                key: "S".to_string(),
            }
        }
    }
}
