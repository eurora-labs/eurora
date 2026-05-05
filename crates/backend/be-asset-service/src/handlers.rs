use std::sync::Arc;

use asset_core::{Asset, CreateAssetRequest};
use auth_core::Claims;
use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose};
use be_asset::CreateAssetInput;
use uuid::Uuid;

use crate::{error::AssetServiceError, service::AppState};

#[tracing::instrument(skip_all, fields(user_sub = %claims.sub, mime_type = %payload.mime_type))]
pub async fn create_asset_handler(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateAssetRequest>,
) -> Result<Response, AssetServiceError> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(AssetServiceError::InvalidUserId)?;

    let content = general_purpose::STANDARD
        .decode(payload.content.as_bytes())
        .map_err(AssetServiceError::InvalidBase64)?;

    let input = CreateAssetInput {
        name: payload.name,
        content,
        mime_type: payload.mime_type,
        metadata: payload.metadata,
        activity_id: payload.activity_id,
    };

    let asset: Asset = state.core.create_asset(input, user_id).await?;

    Ok((StatusCode::CREATED, Json(asset)).into_response())
}
