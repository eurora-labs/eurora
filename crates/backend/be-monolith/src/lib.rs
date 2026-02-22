use axum::http::HeaderValue;
use be_activity_service::{ActivityService, ProtoActivityServiceServer};
use be_asset_service::{AssetService, ProtoAssetServiceServer};
use be_auth_core::JwtConfig;
use be_auth_service::AuthService;
use be_authz::{
    AuthzState, CasbinAuthz, GrpcAuthzLayer, authz_middleware, new_auth_failure_rate_limiter,
};
use be_payment_service::init_payment_service;
use be_remote_db::DatabaseManager;
use be_storage::StorageService;
use be_thread_service::{ProtoThreadServiceServer, ThreadService};
use be_update_service::init_update_service;
use proto_gen::auth::proto_auth_service_server::ProtoAuthServiceServer;
use std::{net::SocketAddr, sync::Arc};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

/// Configuration for running the monolith server.
pub struct ServerConfig {
    pub database_url: String,
    pub grpc_addr: SocketAddr,
    pub http_addr: SocketAddr,
    pub local_mode: bool,
    pub authz_model_path: String,
    pub authz_policy_path: String,
    /// If provided, the encryption key is set on StorageService immediately
    /// at startup (no gRPC SetEncryptionKey call needed).
    pub encryption_key: Option<be_encrypt::MainKey>,
    /// When this receiver gets a value, the server shuts down gracefully.
    pub shutdown: tokio::sync::watch::Receiver<()>,
}

fn build_cors() -> CorsLayer {
    let allowed: Vec<HeaderValue> = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "https://www.eurora-labs.com,https://api.eurora-labs.com".into())
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                return None;
            }
            s.parse::<HeaderValue>().ok()
        })
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed))
        .allow_methods(AllowMethods::mirror_request())
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true)
}

pub async fn run_server(
    config: ServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<ProtoAuthServiceServer<AuthService>>()
        .await;

    if let Some(posthog_key) = std::env::var("POSTHOG_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
    {
        match posthog_rs::init_global(posthog_key.as_str()).await {
            Ok(()) => tracing::info!("PostHog analytics initialized"),
            Err(e) => tracing::warn!("Failed to initialize PostHog: {}", e),
        }
    } else {
        tracing::info!("POSTHOG_API_KEY not set, analytics disabled");
    }

    let db_manager = Arc::new(DatabaseManager::new(&config.database_url).await?);

    let jwt_config = JwtConfig::default();

    let authz = CasbinAuthz::new(&config.authz_model_path, &config.authz_policy_path)
        .await
        .expect("Failed to initialize casbin enforcer");

    let auth_service = AuthService::new(db_manager.clone(), jwt_config.clone());

    let storage_config =
        be_storage::StorageConfig::from_env().expect("Failed to load storage config");
    let storage = Arc::new(
        StorageService::builder()
            .config(storage_config)
            .build()
            .expect("Failed to initialize storage service"),
    );

    // If an encryption key was provided directly (e.g. from euro-server),
    // set it on the storage service immediately instead of waiting for a
    // gRPC SetEncryptionKey call.
    if let Some(key) = config.encryption_key {
        tracing::info!("Encryption key provided at startup, enabling asset encryption");
        storage.set_encryption_key(key);
    }

    let core_asset = Arc::new(be_asset::AssetService::new(
        db_manager.clone(),
        storage.clone(),
    ));
    let activity_service = ActivityService::new(db_manager.clone(), core_asset.clone());
    let assets_service = AssetService::new(db_manager.clone(), storage.clone());
    let (settings_tx, settings_rx) = be_local_settings::settings_channel();

    let thread_service = ThreadService::new(db_manager.clone(), settings_rx);

    tracing::info!("Starting gRPC server at {}", config.grpc_addr);
    tracing::info!("Starting HTTP server at {}", config.http_addr);

    let bucket_name =
        std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "eurora-releases".to_string());

    let update_router = match init_update_service(bucket_name).await {
        Ok(router) => router,
        Err(e) if config.local_mode => {
            tracing::warn!("Update service disabled in local mode: {}", e);
            axum::Router::new()
        }
        Err(e) => {
            tracing::error!("Failed to initialize update service: {}", e);
            return Err(e.into());
        }
    };

    let payment_router = match init_payment_service(db_manager.clone()) {
        Ok(router) => router,
        Err(e) if config.local_mode => {
            tracing::warn!("Payment service disabled in local mode: {}", e);
            axum::Router::new()
        }
        Err(e) => {
            tracing::error!("Failed to initialize payment service: {}", e);
            return Err(e.into());
        }
    };

    let auth_rate_limiter = new_auth_failure_rate_limiter();

    let grpc_authz_layer = GrpcAuthzLayer::new(
        authz.clone(),
        jwt_config.clone(),
        auth_rate_limiter.clone(),
        db_manager.clone(),
    );

    let mut grpc_server = Server::builder()
        .accept_http1(true)
        .layer(build_cors())
        .layer(GrpcWebLayer::new())
        .layer(grpc_authz_layer)
        .add_service(health_service)
        .add_service(ProtoAuthServiceServer::new(auth_service))
        .add_service(ProtoActivityServiceServer::new(activity_service))
        .add_service(ProtoAssetServiceServer::new(assets_service))
        .add_service(ProtoThreadServiceServer::new(thread_service));

    if config.local_mode {
        let local_settings =
            be_local_settings_service::LocalSettingsService::new(storage.clone(), settings_tx);
        grpc_server = grpc_server.add_service(local_settings.into_server());
        tracing::info!("Local mode: registered LocalSettingsService");
    }

    let authz_state = Arc::new(AuthzState::new(authz, jwt_config, auth_rate_limiter));

    let health_route = axum::Router::new().route(
        "/health",
        axum::routing::get(|| async { axum::http::StatusCode::OK }),
    );

    let http_router = update_router
        .merge(payment_router)
        .merge(health_route)
        .layer(build_cors())
        .layer(axum::middleware::from_fn_with_state(
            authz_state,
            authz_middleware,
        ));

    let mut grpc_shutdown = config.shutdown.clone();
    let grpc_future = grpc_server.serve_with_shutdown(config.grpc_addr, async move {
        let _ = grpc_shutdown.changed().await;
        tracing::info!("Shutting down gRPC server...");
    });

    let mut http_shutdown = config.shutdown.clone();
    let http_listener = tokio::net::TcpListener::bind(config.http_addr).await?;
    let http_future = axum::serve(
        http_listener,
        http_router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = http_shutdown.changed().await;
        tracing::info!("Shutting down HTTP server...");
    });

    tokio::select! {
        result = grpc_future => {
            if let Err(e) = result {
                tracing::error!("gRPC server error: {}", e);
                return Err(e.into());
            }
        }
        result = http_future => {
            if let Err(e) = result {
                tracing::error!("HTTP server error: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}
