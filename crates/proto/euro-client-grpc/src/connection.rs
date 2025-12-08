use std::time::Duration;

use anyhow::{Context, Result};
use tonic::transport::Channel;

/// Builder for creating service clients
#[derive(Default)]
pub struct ClientBuilder {
    base_url: String,
    timeout_seconds: u64,
    retries: u32,
}

impl ClientBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:50051".to_string(),
            timeout_seconds: 30,
            retries: 3,
        }
    }

    /// Set the base URL for all services
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set timeout in seconds
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set retry count
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Create a channel for gRPC communication
    pub async fn create_channel(&self) -> Result<Channel> {
        let endpoint = tonic::transport::Endpoint::from_shared(self.base_url.clone())
            .context("Invalid endpoint URL")?
            .timeout(Duration::from_secs(self.timeout_seconds));

        endpoint
            .connect()
            .await
            .context("Failed to connect to gRPC service")
    }
}
