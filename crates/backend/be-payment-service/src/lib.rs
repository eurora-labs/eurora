//! Eurora Payment Service
//!
//! Stripe-based payment service providing checkout sessions, billing portal,
//! subscription management, and webhook handling. Deployed as HTTP routes
//! within the monolith.

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing::debug;

pub mod config;
pub mod error;
pub mod handlers;
pub mod service;
pub mod types;
pub mod webhook;

use service::AppState;

pub fn create_router<H: webhook::WebhookEventHandler>(state: Arc<AppState<H>>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(
            state.config.frontend_url.parse().unwrap_or(
                "http://localhost:5173"
                    .parse()
                    .expect("valid default origin"),
            ),
        ))
        .allow_methods(AllowMethods::mirror_request())
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    Router::new()
        .route("/payment/checkout", post(handlers::create_checkout_session))
        .route("/payment/portal", post(handlers::create_portal_session))
        .route(
            "/payment/subscription",
            get(handlers::get_subscription_status),
        )
        .route("/payment/customers", get(handlers::list_customers))
        .route("/payment/webhook", post(handlers::handle_webhook))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        )
        .with_state(state)
}

/// Initializes the payment service with the default logging-only webhook handler.
///
/// Reads configuration from environment variables (`STRIPE_SECRET_KEY`,
/// `STRIPE_WEBHOOK_SECRET`, etc).
pub fn init_payment_service() -> Result<Router> {
    debug!("Initializing payment service");

    let state = Arc::new(AppState::from_env().context("Failed to create payment service state")?);

    Ok(create_router(state))
}

/// Initializes the payment service with a custom webhook event handler.
///
/// Use this to provide your own provisioning/revocation logic.
pub fn init_payment_service_with_handler<H: webhook::WebhookEventHandler>(
    webhook_handler: Arc<H>,
) -> Result<Router> {
    debug!("Initializing payment service with custom webhook handler");

    let state = Arc::new(
        AppState::from_env_with_handler(webhook_handler)
            .context("Failed to create payment service state")?,
    );

    Ok(create_router(state))
}

pub use config::PaymentConfig;
pub use error::PaymentError;
pub use types::{
    CreateCheckoutRequest, CreateCheckoutResponse, CreatePortalRequest, CreatePortalResponse,
    SubscriptionStatus,
};
pub use webhook::{LoggingWebhookHandler, WebhookEventHandler};
