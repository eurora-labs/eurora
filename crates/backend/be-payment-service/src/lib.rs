use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use be_remote_db::DatabaseManager;
use tower::ServiceBuilder;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};
use tower_http::trace::TraceLayer;

pub mod analytics;
pub mod auth;
pub mod config;
pub mod drainer;
pub mod error;
pub mod handlers;
pub mod provision;
pub mod service;
pub mod types;
pub mod webhook;

use service::AppState;

pub fn create_router(state: Arc<AppState>) -> Result<Router> {
    let checkout_governor = GovernorConfigBuilder::default()
        .per_second(6)
        .burst_size(10)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .context("invalid checkout rate-limiter config")?;

    let authed_governor = GovernorConfigBuilder::default()
        .per_second(30)
        .burst_size(50)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .context("invalid authed rate-limiter config")?;

    let checkout_route = Router::new()
        .route("/payment/checkout", post(handlers::create_checkout_session))
        .layer(GovernorLayer::new(Arc::new(checkout_governor)));

    let authed_routes = Router::new()
        .route("/payment/pricing", get(handlers::get_pricing))
        .route("/payment/portal", post(handlers::create_portal_session))
        .route(
            "/payment/subscription",
            get(handlers::get_subscription_status),
        )
        .route(
            "/payment/checkout-status",
            get(handlers::get_checkout_status),
        )
        .layer(GovernorLayer::new(Arc::new(authed_governor)));

    let webhook_route = Router::new().route("/payment/webhook", post(handlers::handle_webhook));

    Ok(checkout_route
        .merge(authed_routes)
        .merge(webhook_route)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(state))
}

pub struct PaymentService {
    pub router: Router,
    pub provisioner: Arc<StripeBillingProvisioner>,
    pub drainer: drainer::DrainerHandle,
}

pub fn init_payment_service(db: Arc<DatabaseManager>) -> Result<PaymentService> {
    tracing::debug!("Initializing payment service");

    let state =
        Arc::new(AppState::from_env(db.clone()).context("Failed to create payment service state")?);
    let provisioner = state.provisioner.clone();
    let router = create_router(state)?;
    let drainer = drainer::spawn_drainer(db, provisioner.clone());

    Ok(PaymentService {
        router,
        provisioner,
        drainer,
    })
}

pub use config::PaymentConfig;
pub use error::PaymentError;
pub use provision::{ProvisionError, StripeBillingProvisioner};
pub use types::{
    CheckoutStatusResponse, CreateCheckoutRequest, CreateCheckoutResponse, CreatePortalResponse,
    PricingResponse, SubscriptionStatus,
};
