//! Browser Bridge Module
//!
//! This crate provides the gRPC server for browser extension communication.
//! The server is managed by the TimelineManager and runs as long as the manager
//! is alive. When the TimelineManager stops, the server is gracefully shut down.

pub mod proto {
    tonic::include_proto!("browser_bridge");
}

pub mod server;

// Re-export commonly used types
pub use proto::{
    EventFrame, Frame, RegisterFrame, RequestFrame, ResponseFrame,
    browser_bridge_server::BrowserBridgeServer, frame::Kind as FrameKind,
};
pub use server::{BROWSER_BRIDGE_PORT, BrowserBridgeService, RegisteredMessenger};

/// Start the browser bridge gRPC server
///
/// This is a convenience function that initializes the singleton service
/// and starts the server. Should be called when TimelineManager starts.
pub async fn start_browser_bridge_server() {
    let service = BrowserBridgeService::get_or_init().await;
    service.start_frame_handler();
    service.start_server().await;
}

/// Stop the browser bridge gRPC server
///
/// This gracefully shuts down the server and disconnects all native messengers.
/// Should be called when TimelineManager stops.
pub async fn stop_browser_bridge_server() {
    BrowserBridgeService::stop_server().await;
}
