use dotenv::dotenv;
use std::net::SocketAddr;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    // --- Sentry ---
    let _sentry_guard = if cfg!(not(debug_assertions)) {
        std::env::var("SENTRY_MONOLITH_DSN")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|sentry_dsn| {
                let send_pii = std::env::var("SENTRY_SEND_PII")
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                let sentry_debug = std::env::var("SENTRY_DEBUG")
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);

                sentry::init((
                    sentry_dsn,
                    sentry::ClientOptions {
                        release: sentry::release_name!(),
                        traces_sample_rate: 0.0,
                        send_default_pii: send_pii,
                        debug: sentry_debug,
                        ..Default::default()
                    },
                ))
            })
    } else {
        None
    };

    // --- Tracing ---
    let app_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let global_filter = Targets::new()
        .with_default(LevelFilter::WARN)
        .with_target("be_", app_level)
        .with_target("agent_chain", app_level)
        .with_target("hyper", LevelFilter::OFF)
        .with_target("tokio", LevelFilter::OFF);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry::integrations::tracing::layer())
        .with(global_filter)
        .try_init()
        .unwrap();

    // --- Shutdown channel ---
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        tracing::info!("Received CTRL+C, initiating shutdown...");
        let _ = shutdown_tx.send(());
    });

    // --- Server config from environment ---
    let config = be_monolith::ServerConfig {
        database_url: std::env::var("REMOTE_DATABASE_URL")
            .expect("REMOTE_DATABASE_URL environment variable must be set"),
        grpc_addr: std::env::var("MONOLITH_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
            .parse::<SocketAddr>()
            .expect("Invalid MONOLITH_ADDR format"),
        http_addr: std::env::var("HTTP_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
            .parse::<SocketAddr>()
            .expect("Invalid HTTP_ADDR format"),
        local_mode: std::env::var("RUNNING_EURORA_FULLY_LOCAL")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        authz_model_path: std::env::var("AUTHZ_MODEL_PATH")
            .unwrap_or_else(|_| "config/authz/model.conf".to_string()),
        authz_policy_path: std::env::var("AUTHZ_POLICY_PATH")
            .unwrap_or_else(|_| "config/authz/policy.csv".to_string()),
        encryption_key: None, // Docker mode: key comes via gRPC SetEncryptionKey
        shutdown: shutdown_rx,
    };

    be_monolith::run_server(config).await
}
