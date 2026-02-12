mod error;

pub use error::{EndpointError, Result};

use std::sync::RwLock;

use tokio::sync::watch;
use tonic::transport::{Channel, ClientTlsConfig};
use tracing::info;

const DEFAULT_API_URL: &str = "https://api.eurora-labs.com";

/// Centralized API endpoint manager.
///
/// Owns the current API URL and broadcasts a raw `tonic::Channel`
/// via `tokio::sync::watch` whenever the URL changes. Consumers
/// subscribe once and always get the latest channel — they never
/// need to know or care about the URL.
pub struct EndpointManager {
    tx: watch::Sender<Channel>,
    current_url: RwLock<String>,
}

impl EndpointManager {
    pub fn new(initial_url: &str) -> Result<Self> {
        let url = if initial_url.is_empty() {
            DEFAULT_API_URL
        } else {
            initial_url
        };

        let channel = build_channel(url)?;
        let (tx, _) = watch::channel(channel);

        Ok(Self {
            tx,
            current_url: RwLock::new(url.to_owned()),
        })
    }

    pub fn from_env() -> Result<Self> {
        let url = std::env::var("API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
        Self::new(&url)
    }

    pub fn subscribe(&self) -> watch::Receiver<Channel> {
        self.tx.subscribe()
    }

    pub fn set_global_backend_url(&self, url: &str) -> Result<()> {
        let channel = build_channel(url)?;
        *self.current_url.write().unwrap() = url.to_owned();
        self.tx
            .send(channel)
            .map_err(|_| EndpointError::NoSubscribers)?;
        info!("Switched API endpoint to {}", url);
        Ok(())
    }
}

fn build_channel(url: &str) -> Result<Channel> {
    let mut endpoint = Channel::from_shared(url.to_owned())
        .map_err(|e| EndpointError::InvalidUrl(e.to_string()))?;

    if url.starts_with("https://") {
        let tls = ClientTlsConfig::new().with_native_roots();
        endpoint = endpoint.tls_config(tls).map_err(EndpointError::Tls)?;
    }

    Ok(endpoint.connect_lazy())
}
