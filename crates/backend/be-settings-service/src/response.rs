use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use settings_core::{PutSettingsAcceptedResponse, PutSettingsConflictResponse};

/// Outcome of `PUT /settings`. Encapsulates the two HTTP status codes that
/// the handler may return: `200 Accepted` for a successful insert or update,
/// `409 Conflict` when optimistic concurrency rejects the write.
///
/// The HTTP status is the discriminator, so the wire bodies stay as two
/// distinct shapes rather than a tagged enum — clients dispatch on the
/// status code, not a body-level tag.
pub enum PutOutcomeResponse {
    Accepted(PutSettingsAcceptedResponse),
    Conflict(PutSettingsConflictResponse),
}

impl IntoResponse for PutOutcomeResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Accepted(body) => (StatusCode::OK, Json(body)).into_response(),
            Self::Conflict(body) => (StatusCode::CONFLICT, Json(body)).into_response(),
        }
    }
}
