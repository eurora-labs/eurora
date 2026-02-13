mod error;

pub use error::{LocalConfigError, Result};

use std::sync::Arc;

use be_storage::StorageService;
use tonic::{Request, Response, Status};
use tracing::info;

use proto_gen::local_config::{
    SetEncryptionKeyRequest, SetEncryptionKeyResponse,
    proto_local_config_service_server::{ProtoLocalConfigService, ProtoLocalConfigServiceServer},
};

pub use proto_gen::local_config::proto_local_config_service_server::ProtoLocalConfigServiceServer as Server;

#[derive(Debug)]
pub struct LocalConfigService {
    storage: Arc<StorageService>,
}

impl LocalConfigService {
    pub fn new(storage: Arc<StorageService>) -> Self {
        Self { storage }
    }

    pub fn into_server(self) -> ProtoLocalConfigServiceServer<Self> {
        ProtoLocalConfigServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl ProtoLocalConfigService for LocalConfigService {
    async fn set_encryption_key(
        &self,
        request: Request<SetEncryptionKeyRequest>,
    ) -> std::result::Result<Response<SetEncryptionKeyResponse>, Status> {
        let req = request.into_inner();

        let key = be_encrypt::MainKey::from_base64(&req.encryption_key)
            .map_err(LocalConfigError::from)?;

        info!("Encryption key received from client, enabling asset encryption");
        self.storage.set_encryption_key(key);

        Ok(Response::new(SetEncryptionKeyResponse { success: true }))
    }
}
