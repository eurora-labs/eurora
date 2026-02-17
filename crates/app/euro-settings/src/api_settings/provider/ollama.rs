use super::ProviderSettingsTrait;
use crate::error::{Error, Result};
use async_trait::async_trait;
use proto_gen::local_settings as proto;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
pub struct OllamaSettings {
    pub base_url: String,
    pub model: String,
}

impl From<&OllamaSettings> for proto::OllamaSettings {
    fn from(s: &OllamaSettings) -> Self {
        Self {
            base_url: s.base_url.clone(),
            model: s.model.clone(),
        }
    }
}

#[async_trait]
impl ProviderSettingsTrait for OllamaSettings {
    async fn sync(&self, endpoint: &str) -> Result<()> {
        use proto_gen::local_settings::proto_local_settings_service_client::ProtoLocalSettingsServiceClient;

        let mut client = ProtoLocalSettingsServiceClient::connect(endpoint.to_owned())
            .await
            .map_err(|e| Error::Sync(e.to_string()))?;

        client
            .set_provider_settings(proto::SetProviderSettingsRequest {
                settings: Some(proto::ProviderSettings {
                    provider: Some(proto::provider_settings::Provider::Ollama(self.into())),
                }),
            })
            .await
            .map_err(|e| Error::Sync(e.to_string()))?;

        Ok(())
    }
}
