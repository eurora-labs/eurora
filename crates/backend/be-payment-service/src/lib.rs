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
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::debug;

pub mod config;
pub mod error;
pub mod handlers;
pub mod service;
pub mod types;

use service::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
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
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

/// Initializes the payment service and returns an Axum router.
///
/// Reads configuration from environment variables (`STRIPE_SECRET_KEY`,
/// `STRIPE_WEBHOOK_SECRET`, etc).
pub fn init_payment_service() -> Result<Router> {
    debug!("Initializing payment service");

    let state = Arc::new(AppState::from_env().context("Failed to create payment service state")?);

    Ok(create_router(state))
}

pub use config::PaymentConfig;
pub use error::PaymentError;
pub use types::{
    CreateCheckoutRequest, CreateCheckoutResponse, CreatePortalRequest, CreatePortalResponse,
    SubscriptionStatus,
};
