use std::{net::SocketAddr, sync::Arc};

use dotenv::dotenv;
use eur_auth::JwtConfig;
use eur_auth_service::AuthService;
use eur_ocr_service::OcrService;
use eur_prompt_service::PromptService;
use eur_proto::{
    proto_auth_service::proto_auth_service_server::ProtoAuthServiceServer,
    proto_ocr_service::proto_ocr_service_server::ProtoOcrServiceServer,
};
// use eur_proto::proto_prompt_service::proto_prompt_service_server::ProtoPromptServiceServer;
use eur_remote_db::DatabaseManager;
use eur_update_service::init_update_service;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    // Initialize sentry
    let sentry_dsn = std::env::var("SENTRY_MONOLITH_DSN")
        .expect("SENTRY_MONOLITH_DSN environment variable must be set");
    let _guard = sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            send_default_pii: true,
            ..Default::default()
        },
    ));

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<ProtoAuthServiceServer<AuthService>>()
        .await;

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let database_url = std::env::var("REMOTE_DATABASE_URL")
        .expect("REMOTE_DATABASE_URL environment variable must be set");
    let db_manager = Arc::new(DatabaseManager::new(&database_url).await?);

    // Create shared JWT configuration
    let jwt_config = JwtConfig::default();

    let grpc_addr = std::env::var("MONOLITH_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse()
        .expect("Invalid MONOLITH_ADDR format");

    let http_addr: SocketAddr = std::env::var("HTTP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .expect("Invalid HTTP_ADDR format");

    let ocr_service = OcrService::new(Some(jwt_config.clone()));
    let auth_service = AuthService::new(db_manager, Some(jwt_config.clone()));
    let prompt_service = PromptService::new(Some(jwt_config.clone()));

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

    // Create shutdown signal
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Shutting down gracefully...");
    };

    // Start both servers concurrently
    let grpc_server = Server::builder()
        .accept_http1(true)
        .layer(cors)
        .layer(GrpcWebLayer::new())
        .add_service(health_service)
        .add_service(ProtoOcrServiceServer::new(ocr_service))
        .add_service(ProtoAuthServiceServer::new(auth_service))
        .add_service(eur_prompt_service::get_service(prompt_service))
        .serve_with_shutdown(grpc_addr, shutdown_signal);

    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    let http_server = axum::serve(http_listener, update_router).with_graceful_shutdown(async {
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
