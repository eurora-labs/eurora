//! Server-side implementation for the Assets Service.
//!
//! This module contains the gRPC server implementation and is only
//! available when the `server` feature is enabled.

use std::sync::Arc;

use be_asset::AssetService as CoreAssetService;
use be_auth_grpc::Claims;
use be_remote_db::DatabaseManager;
use be_storage::StorageService;
use tonic::{Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

use crate::error::{AssetServiceError, Result};
use proto_gen::asset::{
    ActivityAssetResponse, AssetResponse, CreateAssetRequest, DeleteAssetRequest,
    FindAssetBySha256Request, GetAssetRequest, GetAssetsByActivityIdRequest,
    GetAssetsByMessageIdRequest, LinkAssetToActivityRequest, LinkAssetToMessageRequest,
    ListAssetsRequest, ListAssetsResponse, MessageAssetResponse, UnlinkAssetFromActivityRequest,
    UnlinkAssetFromMessageRequest, UpdateAssetRequest,
};

pub use proto_gen::asset::proto_asset_service_server::{
    ProtoAssetService, ProtoAssetServiceServer,
};

/// The main assets service
#[derive(Debug)]
pub struct AssetService(CoreAssetService);

impl AssetService {
    /// Create a new AssetsService instance
    pub fn new(db: Arc<DatabaseManager>, storage: Arc<StorageService>) -> Self {
        let core_service = CoreAssetService::new(db, storage);
        info!("Creating new AssetsService instance");
        Self(core_service)
    }

    /// Create a new AssetsService with storage configured from environment
    ///
    /// # Errors
    ///
    /// Returns [`AssetServiceError::StorageConfig`] if the storage service
    /// cannot be configured from environment variables.
    pub fn from_env(db: Arc<DatabaseManager>) -> Result<Self> {
        let core_service = CoreAssetService::from_env(db).map_err(AssetServiceError::Asset)?;
        Ok(Self(core_service))
    }
}

/// Extract and validate user ID from request claims
fn extract_user_id<T>(request: &Request<T>) -> Result<Uuid> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(AssetServiceError::MissingClaims)?;

    Uuid::parse_str(&claims.sub)
        .map_err(|_| AssetServiceError::Internal("Invalid user ID".to_string()))
}

#[tonic::async_trait]
impl ProtoAssetService for AssetService {
    async fn create_asset(
        &self,
        request: Request<CreateAssetRequest>,
    ) -> std::result::Result<Response<AssetResponse>, Status> {
        info!("CreateAsset request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();

        let response = self
            .0
            .create_asset(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn get_asset(
        &self,
        request: Request<GetAssetRequest>,
    ) -> std::result::Result<Response<AssetResponse>, Status> {
        info!("GetAsset request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        let response = self
            .0
            .get_asset(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn list_assets(
        &self,
        request: Request<ListAssetsRequest>,
    ) -> std::result::Result<Response<ListAssetsResponse>, Status> {
        info!("ListAssets request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        let response = self
            .0
            .list_assets(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn update_asset(
        &self,
        request: Request<UpdateAssetRequest>,
    ) -> std::result::Result<Response<AssetResponse>, Status> {
        info!("UpdateAsset request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        let response = self
            .0
            .update_asset(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn delete_asset(
        &self,
        request: Request<DeleteAssetRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        info!("DeleteAsset request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        self.0
            .delete_asset(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(()))
    }

    async fn find_asset_by_sha256(
        &self,
        request: Request<FindAssetBySha256Request>,
    ) -> std::result::Result<Response<AssetResponse>, Status> {
        info!("FindAssetBySha256 request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        let response = self
            .0
            .find_asset_by_sha256(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn get_assets_by_message_id(
        &self,
        request: Request<GetAssetsByMessageIdRequest>,
    ) -> std::result::Result<Response<ListAssetsResponse>, Status> {
        info!("GetAssetsByMessageId request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        let response = self
            .0
            .get_assets_by_message_id(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn get_assets_by_activity_id(
        &self,
        request: Request<GetAssetsByActivityIdRequest>,
    ) -> std::result::Result<Response<ListAssetsResponse>, Status> {
        info!("GetAssetsByActivityId request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        let response = self
            .0
            .get_assets_by_activity_id(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn link_asset_to_message(
        &self,
        request: Request<LinkAssetToMessageRequest>,
    ) -> std::result::Result<Response<MessageAssetResponse>, Status> {
        info!("LinkAssetToMessage request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        let response = self
            .0
            .link_asset_to_message(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn unlink_asset_from_message(
        &self,
        request: Request<UnlinkAssetFromMessageRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        info!("UnlinkAssetFromMessage request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        self.0
            .unlink_asset_from_message(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(()))
    }

    async fn link_asset_to_activity(
        &self,
        request: Request<LinkAssetToActivityRequest>,
    ) -> std::result::Result<Response<ActivityAssetResponse>, Status> {
        info!("LinkAssetToActivity request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        let response = self
            .0
            .link_asset_to_activity(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(response))
    }

    async fn unlink_asset_from_activity(
        &self,
        request: Request<UnlinkAssetFromActivityRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        info!("UnlinkAssetFromActivity request received");

        let user_id = extract_user_id(&request).map_err(|e| {
            error!("Authentication failed: {}", e);
            Status::from(e)
        })?;

        let req = request.into_inner();
        self.0
            .unlink_asset_from_activity(req, user_id)
            .await
            .map_err(AssetServiceError::Asset)?;

        Ok(Response::new(()))
    }
}
