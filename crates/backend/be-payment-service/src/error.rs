use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    #[error("Stripe error: {0}")]
    Stripe(#[from] stripe::StripeError),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Webhook signature verification failed")]
    WebhookSignatureInvalid,

    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid value for field: {0}")]
    InvalidField(&'static str),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for PaymentError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            PaymentError::Stripe(stripe::StripeError::Stripe(api_error, code)) => {
                let status = match code {
                    400 => StatusCode::BAD_REQUEST,
                    401 => StatusCode::UNAUTHORIZED,
                    402 => StatusCode::PAYMENT_REQUIRED,
                    404 => StatusCode::NOT_FOUND,
                    429 => StatusCode::TOO_MANY_REQUESTS,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                let message = api_error
                    .message
                    .clone()
                    .unwrap_or_else(|| "Payment processing error".to_string());
                (status, message)
            }
            PaymentError::Stripe(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            PaymentError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            PaymentError::WebhookSignatureInvalid => (StatusCode::BAD_REQUEST, self.to_string()),
            PaymentError::MissingField(_) | PaymentError::InvalidField(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            PaymentError::Config(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            PaymentError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        tracing::error!(%status, error = %self, "Payment service error");

        (status, axum::Json(ErrorBody { error: message })).into_response()
    }
}
