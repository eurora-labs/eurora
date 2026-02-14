use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Extension, Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use be_remote_db::DatabaseManager;
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

pub fn create_router(state: Arc<AppState>) -> Router {
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

    let checkout_governor = GovernorConfigBuilder::default()
        .per_second(6)
        .burst_size(10)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("valid governor config");

    let jwt_config = state.jwt_config.clone();

    let checkout_route = Router::new()
        .route("/payment/checkout", post(handlers::create_checkout_session))
        .layer(GovernorLayer::new(Arc::new(checkout_governor)));

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

    let webhook_route = Router::new().route("/payment/webhook", post(handlers::handle_webhook));

    checkout_route
        .merge(authed_routes)
        .merge(webhook_route)
        .layer(Extension(jwt_config))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        )
        .with_state(state)
}

pub fn init_payment_service(db: Arc<DatabaseManager>) -> Result<Router> {
    debug!("Initializing payment service");

    let state = Arc::new(AppState::from_env(db).context("Failed to create payment service state")?);

    Ok(create_router(state))
}

pub use config::PaymentConfig;
pub use error::PaymentError;
pub use types::{
    CheckoutStatusResponse, CreateCheckoutRequest, CreateCheckoutResponse, CreatePortalResponse,
    SubscriptionStatus,
};
