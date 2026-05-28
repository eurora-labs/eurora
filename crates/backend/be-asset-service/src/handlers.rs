use std::sync::Arc;

use asset_core::{Asset, CreateAssetRequest};
use axum::{
    Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose};
use be_asset::CreateAssetInput;
use be_auth_core::AuthUser;
use uuid::Uuid;

use crate::{error::AssetServiceError, service::AppState};

#[tracing::instrument(skip_all, fields(user_id))]
pub async fn create_asset_handler(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(payload): Json<CreateAssetRequest>,
) -> Result<Response, AssetServiceError> {
    let user_id = user.user_id()?;
    tracing::Span::current().record("user_id", tracing::field::display(user_id));

    let content = general_purpose::STANDARD
        .decode(payload.content.as_bytes())
        .map_err(AssetServiceError::InvalidBase64)?;

    let input = CreateAssetInput {
        name: payload.name,
        content,
        mime_type: payload.mime_type,
        metadata: payload.metadata,
    };

    let asset: Asset = state.core.create_asset(input, user_id).await?;

    Ok((StatusCode::CREATED, Json(asset)).into_response())
}

// Asset paths are uuid-v7 keyed and never rewritten — clients can cache forever.
const ASSET_CACHE_CONTROL: &str = "private, max-age=31536000, immutable";

/// Stream a single asset's raw bytes to the caller.
///
/// The response carries the asset's stored MIME type and a long-lived
/// `Cache-Control` header so clients can fetch icons once per asset id and
/// reuse them indefinitely. Ownership is enforced inside the domain
/// service: an asset owned by a different user surfaces as a clean 404
/// rather than a 403, preserving non-disclosure of foreign asset ids.
#[tracing::instrument(skip_all, fields(user_id, asset_id = %asset_id))]
pub async fn get_asset_bytes_handler(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(asset_id): Path<Uuid>,
) -> Result<Response, AssetServiceError> {
    let user_id = user.user_id()?;
    tracing::Span::current().record("user_id", tracing::field::display(user_id));

    let asset = state.core.get_asset_bytes(asset_id, user_id).await?;

    let headers = [
        (header::CONTENT_TYPE, asset.mime_type),
        (header::CACHE_CONTROL, ASSET_CACHE_CONTROL.to_owned()),
    ];

    Ok((StatusCode::OK, headers, asset.bytes).into_response())
}
