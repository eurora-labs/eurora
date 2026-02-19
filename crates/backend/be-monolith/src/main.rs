use std::{net::SocketAddr, sync::Arc};

use axum::http::HeaderValue;
use be_activity_service::{ActivityService, ProtoActivityServiceServer};
use be_asset_service::{AssetService, ProtoAssetServiceServer};
use be_auth_core::JwtConfig;
use be_auth_service::AuthService;
// use be_authz::{AuthzState, CasbinAuthz, GrpcAuthzLayer, authz_middleware};
use be_authz::{AuthzState, CasbinAuthz, authz_middleware};
use be_payment_service::init_payment_service;
use be_remote_db::DatabaseManager;
use be_storage::StorageService;
use be_thread_service::{ProtoThreadServiceServer, ThreadService};
use be_update_service::init_update_service;
use dotenv::dotenv;
use proto_gen::auth::proto_auth_service_server::ProtoAuthServiceServer;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tracing::{error, info, warn};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

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
                        enable_logs: true,
                        send_default_pii: send_pii,
                        debug: sentry_debug,
                        ..Default::default()
                    },
                ))
            })
    } else {
        None
    };

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<ProtoAuthServiceServer<AuthService>>()
        .await;

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .parse_lossy("be_=debug,agent_chain=debug,hyper=off,tokio=off");

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter.clone()))
        .with(sentry::integrations::tracing::layer().with_filter(filter))
        .try_init()
        .unwrap();

    let database_url = std::env::var("REMOTE_DATABASE_URL")
        .expect("REMOTE_DATABASE_URL environment variable must be set");
    let db_manager = Arc::new(DatabaseManager::new(&database_url).await?);

    let grpc_addr = std::env::var("MONOLITH_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse()
        .expect("Invalid MONOLITH_ADDR format");

    let http_addr: SocketAddr = std::env::var("HTTP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .expect("Invalid HTTP_ADDR format");

    let jwt_config = JwtConfig::default();

    let model_path =
        std::env::var("AUTHZ_MODEL_PATH").unwrap_or_else(|_| "config/authz/model.conf".to_string());
    let policy_path = std::env::var("AUTHZ_POLICY_PATH")
        .unwrap_or_else(|_| "config/authz/policy.csv".to_string());
    let authz = CasbinAuthz::new(&model_path, &policy_path)
        .await
        .expect("Failed to initialize casbin enforcer");

    let auth_service = AuthService::new(db_manager.clone(), jwt_config.clone());

    let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let storage_config =
        be_storage::StorageConfig::from_env().expect("Failed to load storage config");
    let storage = Arc::new(
        StorageService::builder()
            .config(storage_config)
            .build()
            .expect("Failed to initialize storage service"),
    );

    let core_asset = Arc::new(be_asset::AssetService::new(
        db_manager.clone(),
        storage.clone(),
    ));
    let activity_service = ActivityService::new(db_manager.clone(), core_asset.clone());
    let assets_service = AssetService::new(db_manager.clone(), storage.clone());
    let (settings_tx, settings_rx) = be_local_settings::settings_channel();

    let thread_service = ThreadService::new(db_manager.clone(), settings_rx);

    info!("Starting gRPC server at {}", grpc_addr);
    info!("Starting HTTP server at {}", http_addr);

    // let cors = build_cors();

    let bucket_name =
        std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "eurora-releases".to_string());

    let update_router = match init_update_service(bucket_name).await {
        Ok(router) => router,
        Err(e) if local_mode => {
            warn!("Update service disabled in local mode: {}", e);
            axum::Router::new()
        }
        Err(e) => {
            error!("Failed to initialize update service: {}", e);
            return Err(e.into());
        }
    };

    let payment_router = match init_payment_service(db_manager.clone()) {
        Ok(router) => router,
        Err(e) if local_mode => {
            warn!("Payment service disabled in local mode: {}", e);
            axum::Router::new()
        }
        Err(e) => {
            error!("Failed to initialize payment service: {}", e);
            return Err(e.into());
        }
    };

    let shutdown = tokio::signal::ctrl_c();

    // let grpc_authz_layer = GrpcAuthzLayer::new(authz.clone(), jwt_config.clone());
    let grpc_cors = CorsLayer::permissive();

    let mut grpc_router = Server::builder()
        .accept_http1(true)
        .layer(grpc_cors)
        .layer(GrpcWebLayer::new())
        // .layer(cors)
        // .layer(GrpcWebLayer::new())
        // .layer(grpc_authz_layer)
        .add_service(health_service)
        .add_service(ProtoAuthServiceServer::new(auth_service))
        .add_service(ProtoActivityServiceServer::new(activity_service))
        .add_service(ProtoAssetServiceServer::new(assets_service))
        .add_service(ProtoThreadServiceServer::new(thread_service));

    if local_mode {
        let local_settings =
            be_local_settings_service::LocalSettingsService::new(storage.clone(), settings_tx);
        grpc_router = grpc_router.add_service(local_settings.into_server());
        info!("Local mode: registered LocalSettingsService (encryption key will be set by client)");
    }

    let authz_state = Arc::new(AuthzState::new(authz, jwt_config));
    let http_cors = build_cors();

    let health_route = axum::Router::new().route(
        "/health",
        axum::routing::get(|| async { axum::http::StatusCode::OK }),
    );

    let http_router = update_router
        .merge(payment_router)
        .merge(health_route)
        .layer(http_cors)
        .layer(axum::middleware::from_fn_with_state(
            authz_state,
            authz_middleware,
        ));

    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    let http_server = axum::serve(
        http_listener,
        http_router.into_make_service_with_connect_info::<SocketAddr>(),
    );

    let (grpc_result, http_result, _) = tokio::join!(
        grpc_router.serve(grpc_addr),
        http_server.into_future(),
        async {
            shutdown
                .await
                .expect("Failed to install CTRL+C signal handler");
            info!("Shutdown signal received");
        }
    );

    if let Err(e) = grpc_result {
        error!("gRPC server error: {}", e);
    }
    if let Err(e) = http_result {
        error!("HTTP server error: {}", e);
    }

    Ok(())
}
