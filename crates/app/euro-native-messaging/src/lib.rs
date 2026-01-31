use anyhow::Result;
pub use tonic::transport::Channel;

pub mod parent_pid;
pub mod server;
pub mod types;
pub mod utils;

pub use server::BrowserBridgeClient;

pub const MAX_FRAME_SIZE: usize = 8 * 1024 * 1024;

pub use types::*;

/// The port for the browser bridge gRPC server (running in euro-activity)
pub const PORT: &str = "1431";

/// Create a browser bridge client to connect to the euro-activity server
pub async fn create_browser_bridge_client() -> Result<BrowserBridgeClient<Channel>> {
    Ok(BrowserBridgeClient::connect(format!("http://[::1]:{}", PORT)).await?)
}
