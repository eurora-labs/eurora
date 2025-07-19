use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{Method, header},
    routing::{get, post},
};
use eur_remote_db::DatabaseManager;
use eur_stripe_service::{Config, StripeService, handlers, webhooks};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "eur_stripe_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!(
        "Starting Stripe service on {}:{}",
        config.server.host,
        config.server.port
    );

    // Initialize database connection
    let db = DatabaseManager::new(&config.database.url).await?;
    tracing::info!("Database connection established");

    // Initialize Stripe service
    let stripe_service = StripeService::new(config.clone(), db);

    // Build our application with routes
    let app = create_app(stripe_service);

    // Create socket address
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Stripe service listening on {}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn create_app(stripe_service: StripeService) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // Customer routes
        .route("/customers", post(handlers::create_customer))
        .route("/customers/{id}", get(handlers::get_customer))
        // Product routes
        .route("/products", post(handlers::create_product))
        .route("/products/{id}", get(handlers::get_product))
        // Price routes
        .route("/prices", post(handlers::create_price))
        // Subscription routes
        .route("/subscriptions", post(handlers::create_subscription))
        // Payment Intent routes
        .route("/payment-intents", post(handlers::create_payment_intent))
        // Checkout Session routes
        .route(
            "/checkout-sessions",
            post(handlers::create_checkout_session),
        )
        // Webhook endpoint
        .route("/webhooks/stripe", post(webhooks::handle_webhook))
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
                )
                .layer(DefaultBodyLimit::max(1024 * 1024)), // 1MB limit
        )
        .with_state(stripe_service)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let config = Config::default();
        let db = DatabaseManager::new(&config.database.url).await.unwrap();
        let service = StripeService::new(config, db);
        let app = create_app(service);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
