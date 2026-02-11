//! Eurora Payment Service
//!
//! Stripe-based payment service providing checkout sessions, billing portal,
//! subscription management, and webhook handling. Deployed as HTTP routes
//! within the monolith.

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Extension, Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing::debug;

pub mod auth;
pub mod config;
pub mod error;
pub mod handlers;
pub mod service;
pub mod types;
pub mod webhook;

use service::AppState;

/// Creates the payment service [`Router`].
///
/// # Security
///
/// All endpoints except `POST /payment/webhook` require a valid JWT access
/// token in the `Authorization: Bearer <token>` header. The token is validated
/// using the [`be_auth_core::JwtConfig`] that is injected as an axum extension.
///
/// `POST /payment/webhook` uses Stripe webhook signature verification instead.
///
/// `POST /payment/checkout` is additionally rate-limited to 10 requests per
/// minute per IP to prevent abuse.
pub fn create_router<H: webhook::WebhookEventHandler>(state: Arc<AppState<H>>) -> Router {
    // FRONTEND_URL is validated during PaymentConfig::from_env(), so this
    // parse cannot fail at runtime.
    let origin = state
        .config
        .frontend_url
        .parse()
        .expect("FRONTEND_URL was validated during config loading");

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(origin))
        .allow_methods(AllowMethods::mirror_request())
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    // Rate limit: 10 requests per 60 seconds per client IP on checkout.
    // SmartIpKeyExtractor checks X-Forwarded-For / X-Real-IP headers first,
    // falling back to peer IP — works correctly behind a reverse proxy.
    let checkout_governor = GovernorConfigBuilder::default()
        .per_second(6)
        .burst_size(10)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("valid governor config");

    let jwt_config = state.jwt_config.clone();

    // Checkout route with rate limiting
    let checkout_route = Router::new()
        .route("/payment/checkout", post(handlers::create_checkout_session))
        .layer(GovernorLayer::new(Arc::new(checkout_governor)));

    // Other authenticated routes (no extra rate limiting)
    let authed_routes = Router::new()
        .route("/payment/portal", post(handlers::create_portal_session))
        .route(
            "/payment/subscription",
            get(handlers::get_subscription_status),
        )
        .route(
            "/payment/checkout-status",
            get(handlers::get_checkout_status),
        );

    // Webhook route — no JWT auth, uses Stripe signature verification
    let webhook_route = Router::new().route("/payment/webhook", post(handlers::handle_webhook));

    checkout_route
        .merge(authed_routes)
        .merge(webhook_route)
        .layer(Extension(jwt_config))
        .layer(DefaultBodyLimit::max(1024 * 1024)) // 1 MB
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
    CheckoutStatusResponse, CreateCheckoutRequest, CreateCheckoutResponse, CreatePortalResponse,
    SubscriptionStatus,
};
pub use webhook::{LoggingWebhookHandler, WebhookEventHandler};
