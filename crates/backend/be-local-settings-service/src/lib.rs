mod error;

pub use error::{LocalSettingsError, Result};

use std::sync::Arc;

use be_local_settings::{
    ProviderSettings, SettingsSender,
    proto::{
        self, SetEncryptionKeyRequest, SetEncryptionKeyResponse, SetProviderSettingsRequest,
        SetProviderSettingsResponse,
        proto_local_settings_service_server::{
            ProtoLocalSettingsService, ProtoLocalSettingsServiceServer,
        },
        provider_settings::Provider,
    },
};
use be_storage::StorageService;
use tonic::{Request, Response, Status};
use tracing::info;

pub struct LocalSettingsService {
    storage: Arc<StorageService>,
    settings_tx: SettingsSender,
}

impl LocalSettingsService {
    pub fn new(storage: Arc<StorageService>, settings_tx: SettingsSender) -> Self {
        Self {
            storage,
            settings_tx,
        }
    }

    pub fn into_server(self) -> ProtoLocalSettingsServiceServer<Self> {
        ProtoLocalSettingsServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl ProtoLocalSettingsService for LocalSettingsService {
    async fn set_encryption_key(
        &self,
        request: Request<SetEncryptionKeyRequest>,
    ) -> std::result::Result<Response<SetEncryptionKeyResponse>, Status> {
        let req = request.into_inner();

        let key = be_encrypt::MainKey::from_base64(&req.encryption_key)
            .map_err(LocalSettingsError::from)?;

        info!("Encryption key received from client, enabling asset encryption");
        self.storage.set_encryption_key(key);

        Ok(Response::new(SetEncryptionKeyResponse { success: true }))
    }

    async fn set_provider_settings(
        &self,
        request: Request<SetProviderSettingsRequest>,
    ) -> std::result::Result<Response<SetProviderSettingsResponse>, Status> {
        let proto_provider = request
            .into_inner()
            .settings
            .and_then(|s| s.provider)
            .ok_or_else(|| Status::invalid_argument("provider settings are required"))?;

        let provider: ProviderSettings = match proto_provider {
            Provider::Ollama(p) => {
                p.try_into()
                    .map(ProviderSettings::Ollama)
                    .map_err(|e: url::ParseError| {
                        LocalSettingsError::InvalidProviderSettings(e.to_string())
                    })?
            }
            Provider::Openai(p) => {
                p.try_into()
                    .map(ProviderSettings::OpenAI)
                    .map_err(|e: url::ParseError| {
                        LocalSettingsError::InvalidProviderSettings(e.to_string())
                    })?
            }
        };

        info!("Provider settings updated: {:?}", provider);

        let response_proto: proto::ProviderSettings = (&provider).into();

        self.settings_tx
            .send(Some(provider))
            .map_err(|_| Status::internal("all settings subscribers dropped"))?;

        Ok(Response::new(SetProviderSettingsResponse {
            settings: Some(response_proto),
        }))
    }
}
