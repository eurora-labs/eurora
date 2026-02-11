use std::{net::SocketAddr, sync::Arc};

use be_activity_service::{ActivityService, ProtoActivityServiceServer};
use be_asset_service::{AssetService, ProtoAssetServiceServer};
use be_auth_service::AuthService;
use be_conversation_service::{ConversationService, ProtoConversationServiceServer};
use dotenv::dotenv;
use proto_gen::auth::proto_auth_service_server::ProtoAuthServiceServer;
// use euro_proto::proto_prompt_service::proto_prompt_service_server::ProtoPromptServiceServer;
use be_auth_grpc::JwtInterceptor;
use be_payment_service::init_payment_service;
use be_remote_db::DatabaseManager;
use be_update_service::init_update_service;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    // Initialize sentry if running in production
    if cfg!(not(debug_assertions)) {
        let sentry_dsn =
            std::env::var("SENTRY_MONOLITH_DSN").expect("SENTRY_MONOLITH_DSN must be set");
        let _guard = sentry::init((
            sentry_dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 0.0,
                enable_logs: true,
                send_default_pii: true, // during closed beta all metrics are non-anonymous
                debug: true,
                ..Default::default()
            },
        ));
    }

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<ProtoAuthServiceServer<AuthService>>()
        .await;

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into()) // anything not listed → WARN
        .parse_lossy("be_=debug,hyper=off,tokio=off"); // keep yours, silence deps

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter.clone()))
        .with(sentry::integrations::tracing::layer().with_filter(filter))
        .try_init()
        .unwrap();

    // let subscriber = FmtSubscriber::builder()
    //     .with_max_level(Level::INFO)
    //     .finish();
    // tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

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

    let jwt_interceptor = JwtInterceptor::default();

    let auth_service = AuthService::new(db_manager.clone(), jwt_interceptor.get_config().clone());
    let activity_service = ActivityService::from_env(db_manager.clone())
        .expect("Failed to initialize activity service");
    let assets_service =
        AssetService::from_env(db_manager.clone()).expect("Failed to initialize assets service");
    let conversation_service = ConversationService::from_env(db_manager.clone())
        .expect("Failed to initialize conversation service");

    info!("Starting gRPC server at {}", grpc_addr);
    info!("Starting HTTP server at {}", http_addr);

    let cors = CorsLayer::permissive();

    // Initialize update service
    let bucket_name =
        std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "eurora-releases".to_string());

    let update_router = match init_update_service(bucket_name).await {
        Ok(router) => router,
        Err(e) => {
            error!("Failed to initialize update service: {}", e);
            return Err(e.into());
        }
    };

    // Initialize payment service
    let payment_router = match init_payment_service() {
        Ok(router) => router,
        Err(e) => {
            error!("Failed to initialize payment service: {}", e);
            return Err(e.into());
        }
    };

    // Create shutdown signal
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        debug!("Shutting down gracefully...");
    };

    // Start both servers concurrently
    let grpc_server = Server::builder()
        .accept_http1(true)
        .layer(cors)
        .layer(GrpcWebLayer::new())
        .add_service(health_service)
        .add_service(ProtoAuthServiceServer::new(auth_service))
        .add_service(ProtoActivityServiceServer::with_interceptor(
            activity_service,
            jwt_interceptor.clone(),
        ))
        .add_service(ProtoAssetServiceServer::with_interceptor(
            assets_service,
            jwt_interceptor.clone(),
        ))
        .add_service(ProtoConversationServiceServer::with_interceptor(
            conversation_service,
            jwt_interceptor.clone(),
        ))
        .serve_with_shutdown(grpc_addr, shutdown_signal);

    let http_router = update_router.merge(payment_router);

    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    let http_server = axum::serve(http_listener, http_router).with_graceful_shutdown(async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    });

    // Run both servers concurrently
    tokio::select! {
        result = grpc_server => {
            if let Err(e) = result {
                error!("gRPC server error: {}", e);
                return Err(e.into());
            }
        }
        result = http_server => {
            if let Err(e) = result {
                error!("HTTP server error: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}
