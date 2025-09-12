use anyhow::Result;
pub use eur_proto::ipc::tauri_ipc_client::TauriIpcClient;
pub use tonic::transport::Channel;

mod types;

pub mod asset_context;
pub mod asset_converter;
pub mod server;
pub mod snapshot_context;
pub mod snapshot_converter;

pub use asset_context::{ArticleState, PdfState, YoutubeState};
pub use snapshot_context::YoutubeSnapshot;

// Define the port as a constant to ensure consistency
pub const PORT: &str = "1421";

pub async fn create_grpc_ipc_client() -> Result<TauriIpcClient<Channel>> {
    Ok(TauriIpcClient::connect(format!("http://[::1]:{}", PORT)).await?)
}
