use std::sync::Arc;

use asset_core::{Asset, CreateAssetRequest};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose};
use be_asset::CreateAssetInput;
use be_auth_core::AuthUser;

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
        activity_id: payload.activity_id,
    };

    let asset: Asset = state.core.create_asset(input, user_id).await?;

    Ok((StatusCode::CREATED, Json(asset)).into_response())
}
