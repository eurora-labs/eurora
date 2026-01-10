mod client;
mod interceptor;
mod manager;

pub mod proto {
    tonic::include_proto!("auth_service");
}

pub use auth_core::*;
pub use client::*;
pub use interceptor::*;
pub use manager::*;

use anyhow::{Ok, Result, anyhow};
use tonic::transport::{Channel, ClientTlsConfig};

pub async fn get_secure_channel() -> Result<Channel> {
    let base_url =
        std::env::var("API_BASE_URL").unwrap_or("https://api.eurora-labs.com".to_string());
    let tls = ClientTlsConfig::new().with_native_roots();
    let channel = Channel::from_shared(base_url.clone())?
        .tls_config(tls)?
        .connect()
        .await
        .map_err(|e| anyhow!("Failed to connect to url: {}", e))?;

    Ok(channel)
}
