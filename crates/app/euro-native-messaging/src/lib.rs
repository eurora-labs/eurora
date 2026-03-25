use anyhow::Result;
pub use tonic::transport::Channel;

pub mod parent_pid;
pub mod server;
pub mod types;
pub mod utils;

pub use server::BrowserBridgeClient;

pub const MAX_FRAME_SIZE: usize = 1024 * 1024 * 1024;

pub use types::*;

pub const PORT: &str = "1431";

pub async fn create_browser_bridge_client() -> Result<BrowserBridgeClient<Channel>> {
    Ok(
        BrowserBridgeClient::connect(format!("http://[::1]:{}", PORT))
            .await?
            .max_decoding_message_size(1024 * 1024 * 1024)
            .max_encoding_message_size(1024 * 1024 * 1024),
    )
}
