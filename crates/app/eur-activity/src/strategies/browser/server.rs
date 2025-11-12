//! Browser strategy gRPC server configuration
//!
//! This module contains the gRPC server implementation for receiving
//! activity switch events from browser extensions.
use eur_proto::nm_ipc::{SwitchActivityRequest, native_messaging_ipc_server::NativeMessagingIpc};
use std::net::ToSocketAddrs;
use std::sync::OnceLock;
use tokio::sync::broadcast;
use tonic::{Status, transport::Server};
use tracing::{debug, error, info, warn};

/// Port number for the persistent gRPC server
pub const PORT: &str = "1422";

/// Global singleton for the gRPC server instance
static GRPC_SERVER: OnceLock<NativeMessagingServer> = OnceLock::new();

/// Global singleton for the server task handle
static SERVER_TASK: OnceLock<tokio::task::JoinHandle<()>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct NativeMessagingServer {
    pub activity_event_tx: broadcast::Sender<SwitchActivityRequest>,
}

impl NativeMessagingServer {
    pub fn new() -> Self {
        let activity_event_tx = broadcast::channel(100).0;
        Self { activity_event_tx }
    }

    /// Get a receiver for activity events
    pub fn subscribe(&self) -> broadcast::Receiver<SwitchActivityRequest> {
        self.activity_event_tx.subscribe()
    }
}

#[tonic::async_trait]
impl NativeMessagingIpc for NativeMessagingServer {
    async fn switch_activity(
        &self,
        request: tonic::Request<eur_proto::nm_ipc::SwitchActivityRequest>,
    ) -> Result<tonic::Response<eur_proto::nm_ipc::SwitchActivityResponse>, tonic::Status> {
        info!("Received switch activity request via persistent gRPC server");
        let req = request.into_inner();

        // Validate the URL is not empty
        if req.url.is_empty() {
            return Err(Status::invalid_argument("URL cannot be empty"));
        }

        info!("Processing activity switch for URL: {}", req.url.clone());

        let _ = self.activity_event_tx.send(req);

        // Return success response
        Ok(tonic::Response::new(
            eur_proto::nm_ipc::SwitchActivityResponse {},
        ))
    }
}

/// Ensures the gRPC server is running, initializing it only once
pub async fn ensure_grpc_server_running() {
    // Initialize the server instance if not already done
    let server = GRPC_SERVER.get_or_init(|| {
        info!("Initializing gRPC server instance");
        NativeMessagingServer::new()
    });

    // Initialize the server task if not already running
    SERVER_TASK.get_or_init(|| {
        info!("Starting persistent gRPC server on port {}", PORT);

        let server = server.clone();

        tokio::spawn(async move {
            let addr = format!("[::1]:{}", PORT)
                .to_socket_addrs()
                .expect("Failed to parse gRPC server address")
                .next()
                .expect("Failed to resolve gRPC server address");

            info!("gRPC server listening at {}", addr);

            // Create the gRPC service from our implementation
            let svc = eur_proto::nm_ipc::native_messaging_ipc_server::NativeMessagingIpcServer::new(
                server,
            );

            // Build and serve the gRPC server
            match Server::builder().add_service(svc).serve(addr).await {
                Ok(_) => {
                    info!("gRPC server stopped gracefully");
                }
                Err(e) => {
                    error!("gRPC server terminated with error: {}", e);
                }
            }
        })
    });

    debug!("gRPC server initialization complete");
}

/// Get a reference to the global gRPC server instance
pub fn get_server() -> &'static NativeMessagingServer {
    GRPC_SERVER.get_or_init(|| {
        debug!("Lazily initializing gRPC server instance");
        NativeMessagingServer::new()
    })
}
