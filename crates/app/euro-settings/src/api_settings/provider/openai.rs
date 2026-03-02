use super::ProviderSettingsTrait;
use crate::error::{Error, Result};
use async_trait::async_trait;
use euro_secret::{ExposeSecret, SecretString, secret};
use proto_gen::local_settings as proto;
use serde::{Deserialize, Serialize};
use specta::Type;

const OPENAI_API_KEY_HANDLE: &str = "OPENAI_API_KEY";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
pub struct OpenAISettings {
    pub base_url: String,
    pub model: String,
    pub title_model: Option<String>,
}

impl OpenAISettings {
    fn api_key() -> Result<Option<SecretString>> {
        secret::retrieve(OPENAI_API_KEY_HANDLE).map_err(|e| Error::Secret(e.to_string()))
    }

    pub fn set_api_key(api_key: &str) -> Result<()> {
        secret::persist(
            OPENAI_API_KEY_HANDLE,
            &SecretString::from(api_key.to_owned()),
        )
        .map_err(|e| Error::Secret(e.to_string()))
    }

    fn to_proto(&self) -> Result<proto::OpenAiSettings> {
        let api_key = Self::api_key()?
            .map(|s| s.expose_secret().to_owned())
            .unwrap_or_default();
        Ok(proto::OpenAiSettings {
            base_url: self.base_url.clone(),
            api_key,
            model: self.model.clone(),
            title_model: self.title_model.clone().unwrap_or_default(),
        })
    }
}

#[async_trait]
impl ProviderSettingsTrait for OpenAISettings {
    async fn sync(&self, endpoint: &str) -> Result<()> {
        use proto_gen::local_settings::proto_local_settings_service_client::ProtoLocalSettingsServiceClient;

        let mut client = ProtoLocalSettingsServiceClient::connect(endpoint.to_owned())
            .await
            .map_err(|e| Error::Sync(e.to_string()))?;

        client
            .set_provider_settings(proto::SetProviderSettingsRequest {
                settings: Some(proto::ProviderSettings {
                    provider: Some(proto::provider_settings::Provider::Openai(self.to_proto()?)),
                }),
            })
            .await
            .map_err(|e| Error::Sync(e.to_string()))?;

        Ok(())
    }
}
