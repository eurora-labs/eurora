use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StripeServiceError>;

#[derive(Error, Debug)]
pub enum StripeServiceError {
    #[error("Stripe API error: {0}")]
    StripeError(#[from] stripe::StripeError),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),

    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Customer not found: {0}")]
    CustomerNotFound(String),

    #[error("Product not found: {0}")]
    ProductNotFound(String),

    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),

    #[error("Payment intent not found: {0}")]
    PaymentIntentNotFound(String),

    #[error("Webhook verification failed: {0}")]
    WebhookVerificationFailed(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for StripeServiceError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            StripeServiceError::StripeError(e) => {
                tracing::error!("Stripe API error: {:?}", e);
                (StatusCode::BAD_REQUEST, format!("Stripe error: {}", e))
            }
            StripeServiceError::DatabaseError(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            StripeServiceError::SerializationError(e) => {
                tracing::error!("Serialization error: {:?}", e);
                (StatusCode::BAD_REQUEST, "Invalid JSON format".to_string())
            }
            StripeServiceError::ConfigError(e) => {
                tracing::error!("Configuration error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Configuration error".to_string(),
                )
            }
            StripeServiceError::EnvVarError(e) => {
                tracing::error!("Environment variable error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Configuration error".to_string(),
                )
            }
            StripeServiceError::InvalidRequest(msg) => {
                tracing::warn!("Invalid request: {}", msg);
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            StripeServiceError::CustomerNotFound(id) => {
                tracing::warn!("Customer not found: {}", id);
                (StatusCode::NOT_FOUND, format!("Customer not found: {}", id))
            }
            StripeServiceError::ProductNotFound(id) => {
                tracing::warn!("Product not found: {}", id);
                (StatusCode::NOT_FOUND, format!("Product not found: {}", id))
            }
            StripeServiceError::SubscriptionNotFound(id) => {
                tracing::warn!("Subscription not found: {}", id);
                (
                    StatusCode::NOT_FOUND,
                    format!("Subscription not found: {}", id),
                )
            }
            StripeServiceError::PaymentIntentNotFound(id) => {
                tracing::warn!("Payment intent not found: {}", id);
                (
                    StatusCode::NOT_FOUND,
                    format!("Payment intent not found: {}", id),
                )
            }
            StripeServiceError::WebhookVerificationFailed(msg) => {
                tracing::error!("Webhook verification failed: {}", msg);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Webhook verification failed: {}", msg),
                )
            }
            StripeServiceError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            StripeServiceError::Unauthorized(msg) => {
                tracing::warn!("Unauthorized: {}", msg);
                (StatusCode::UNAUTHORIZED, msg.clone())
            }
            StripeServiceError::BadRequest(msg) => {
                tracing::warn!("Bad request: {}", msg);
                (StatusCode::BAD_REQUEST, msg.clone())
            }
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": self.error_type(),
            }
        }));

        (status, body).into_response()
    }
}

impl StripeServiceError {
    fn error_type(&self) -> &'static str {
        match self {
            StripeServiceError::StripeError(_) => "stripe_error",
            StripeServiceError::DatabaseError(_) => "database_error",
            StripeServiceError::SerializationError(_) => "serialization_error",
            StripeServiceError::ConfigError(_) => "config_error",
            StripeServiceError::EnvVarError(_) => "env_var_error",
            StripeServiceError::InvalidRequest(_) => "invalid_request",
            StripeServiceError::CustomerNotFound(_) => "customer_not_found",
            StripeServiceError::ProductNotFound(_) => "product_not_found",
            StripeServiceError::SubscriptionNotFound(_) => "subscription_not_found",
            StripeServiceError::PaymentIntentNotFound(_) => "payment_intent_not_found",
            StripeServiceError::WebhookVerificationFailed(_) => "webhook_verification_failed",
            StripeServiceError::InternalError(_) => "internal_error",
            StripeServiceError::Unauthorized(_) => "unauthorized",
            StripeServiceError::BadRequest(_) => "bad_request",
        }
    }
}
