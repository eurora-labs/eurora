mod clients;
use anyhow::{Ok, Result, anyhow};
pub use clients::*;
use tonic::transport::{Channel, ClientTlsConfig};

async fn get_secure_channel(base_url: String) -> Result<Option<Channel>> {
    let tls = ClientTlsConfig::new().with_native_roots();
    let channel = Channel::from_shared(base_url.clone())?
        .tls_config(tls)?
        .connect()
        .await
        .map_err(|e| anyhow!("Failed to connect to url: {}", e))?;

    Ok(Some(channel))
}
