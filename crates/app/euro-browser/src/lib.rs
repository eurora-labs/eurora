pub mod proto {
    tonic::include_proto!("browser_bridge");
}

pub mod server;

pub use proto::{
    EventFrame, Frame, RegisterFrame, RequestFrame, ResponseFrame,
    browser_bridge_server::BrowserBridgeServer, frame::Kind as FrameKind,
};
pub use server::{BROWSER_BRIDGE_PORT, BrowserBridgeService, RegisteredMessenger};

pub async fn start_browser_bridge_server() {
    let service = BrowserBridgeService::get_or_init().await;
    service.start_frame_handler();
    service.start_server().await;
}

pub async fn stop_browser_bridge_server() {
    BrowserBridgeService::stop_server().await;
}
