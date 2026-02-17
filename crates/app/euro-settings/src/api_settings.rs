use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::Result;

mod provider;

pub use provider::*;

const CLOUD_ENDPOINT: &str = "https://api.eurora-labs.com";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct APISettings {
    pub endpoint: String,
    pub provider: Option<ProviderSettings>,
}

impl APISettings {
    /// Sync provider settings to the local backend.
    ///
    /// If the endpoint is the cloud endpoint, provider settings are cleared
    /// since the cloud backend manages its own provider configuration.
    pub async fn sync(&mut self) -> Result<()> {
        if self.endpoint == CLOUD_ENDPOINT {
            self.provider = None;
            return Ok(());
        }

        if let Some(ref provider) = self.provider {
            provider.sync(&self.endpoint).await?;
        }

        Ok(())
    }
}
