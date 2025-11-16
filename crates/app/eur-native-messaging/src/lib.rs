use anyhow::Result;
pub use tonic::transport::Channel;

pub mod server;
pub mod types;
pub mod utils;

pub use server::BrowserBridgeClient;

// pub use server_o::IncomingMessage;
pub use types::*;

// Define the port as a constant to ensure consistency
pub const PORT: &str = "1421";

pub async fn create_browser_bridge_client() -> Result<BrowserBridgeClient<Channel>> {
    Ok(BrowserBridgeClient::connect(format!("http://[::1]:{}", PORT)).await?)
}
