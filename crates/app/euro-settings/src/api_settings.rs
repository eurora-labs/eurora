use serde::{Deserialize, Serialize};
use specta::Type;

mod provider;

pub use provider::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct APISettings {
    pub endpoint: String,
    pub provider: Option<ProviderSettings>,
}
