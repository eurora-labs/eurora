use std::sync::Arc;

use be_asset::AssetService as CoreAssetService;
use be_authz::{extract_claims, parse_user_id};
use be_remote_db::DatabaseManager;
use be_storage::StorageService;
use tonic::{Request, Response, Status};

use crate::error::{AssetServiceError, Result};
use proto_gen::asset::{AssetResponse, CreateAssetRequest};

pub use proto_gen::asset::proto_asset_service_server::{
    ProtoAssetService, ProtoAssetServiceServer,
};

#[derive(Debug)]
pub struct AssetService(CoreAssetService);

impl AssetService {
    pub fn new(db: Arc<DatabaseManager>, storage: Arc<StorageService>) -> Self {
        let core_service = CoreAssetService::new(db, storage);
        tracing::info!("Creating new AssetsService instance");
        Self(core_service)
    }

    pub fn from_env(db: Arc<DatabaseManager>) -> Result<Self> {
        let core_service = CoreAssetService::from_env(db).map_err(AssetServiceError::Asset)?;
        Ok(Self(core_service))
    }
}

#[tonic::async_trait]
impl ProtoAssetService for AssetService {
    async fn create_asset(
        &self,
        request: Request<CreateAssetRequest>,
    ) -> std::result::Result<Response<AssetResponse>, Status> {
        tracing::info!("CreateAsset request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let response = self
            .0
            .create_asset(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }
}
