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
    },
};
use be_storage::StorageService;
use tonic::{Request, Response, Status};

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

        tracing::info!("Encryption key received from client, enabling asset encryption");
        self.storage.set_encryption_key(key);

        Ok(Response::new(SetEncryptionKeyResponse { success: true }))
    }

    async fn set_provider_settings(
        &self,
        request: Request<SetProviderSettingsRequest>,
    ) -> std::result::Result<Response<SetProviderSettingsResponse>, Status> {
        let proto_settings = request
            .into_inner()
            .settings
            .ok_or_else(|| Status::invalid_argument("provider settings are required"))?;

        let provider: ProviderSettings = proto_settings
            .try_into()
            .map_err(LocalSettingsError::from)?;

        tracing::info!("Provider settings updated: {:?}", provider);

        let response_proto: proto::ProviderSettings = (&provider).into();

        self.settings_tx
            .send(Some(provider))
            .map_err(|_| Status::internal("all settings subscribers dropped"))?;

        Ok(Response::new(SetProviderSettingsResponse {
            settings: Some(response_proto),
        }))
    }
}
