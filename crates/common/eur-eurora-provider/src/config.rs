//! Configuration types for gRPC providers.

use std::time::Duration;

use ferrous_llm_core::config::ProviderConfig;
use serde::{Deserialize, Serialize};
use url::Url;

/// Configuration for gRPC providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EuroraConfig {
    /// The gRPC server endpoint URL
    pub endpoint: Url,

    /// Optional authentication token
    pub auth_token: Option<String>,

    /// Request timeout duration
    pub timeout: Option<Duration>,

    /// Maximum message size for requests (in bytes)
    pub max_request_size: Option<usize>,

    /// Maximum message size for responses (in bytes)
    pub max_response_size: Option<usize>,

    /// Whether to use TLS
    pub use_tls: bool,

    /// Optional TLS domain name for verification
    pub tls_domain: Option<String>,

    /// Connection timeout
    pub connect_timeout: Option<Duration>,

    /// Keep-alive interval
    pub keep_alive_interval: Option<Duration>,

    /// Keep-alive timeout
    pub keep_alive_timeout: Option<Duration>,

    /// Whether to enable keep-alive while idle
    pub keep_alive_while_idle: bool,

    /// Maximum number of concurrent requests
    pub max_concurrent_requests: Option<usize>,

    /// User agent string
    pub user_agent: Option<String>,
}

impl Default for EuroraConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("http://localhost:50051").unwrap(),
            auth_token: None,
            timeout: Some(Duration::from_secs(30)),
            max_request_size: Some(4 * 1024 * 1024),  // 4MB
            max_response_size: Some(4 * 1024 * 1024), // 4MB
            use_tls: false,
            tls_domain: None,
            connect_timeout: Some(Duration::from_secs(10)),
            keep_alive_interval: Some(Duration::from_secs(30)),
            keep_alive_timeout: Some(Duration::from_secs(5)),
            keep_alive_while_idle: true,
            max_concurrent_requests: Some(100),
            user_agent: Some("ferrous-llm-grpc/0.2.0".to_string()),
        }
    }
}

impl ProviderConfig for EuroraConfig {
    type Provider = crate::provider::EuroraChatProvider;

    fn build(self) -> Result<Self::Provider, ferrous_llm_core::error::ConfigError> {
        use ferrous_llm_core::error::ConfigError;

        self.validate()?;

        // Note: This is a synchronous build method, but GrpcChatProvider::new is async
        // In practice, this would need to be handled differently, perhaps with a builder pattern
        // For now, we'll return an error indicating async construction is needed
        Err(ConfigError::validation_failed(
            "EuroraChatProvider requires async construction. Use EuroraChatProvider::new(config).await instead",
        ))
    }

    fn validate(&self) -> Result<(), ferrous_llm_core::error::ConfigError> {
        use ferrous_llm_core::error::ConfigError;

        // Validate endpoint
        if self.endpoint.scheme() != "http" && self.endpoint.scheme() != "https" {
            return Err(ConfigError::invalid_value(
                "endpoint",
                "Endpoint must use http or https scheme",
            ));
        }

        // Validate TLS configuration
        if self.use_tls && self.endpoint.scheme() != "https" {
            return Err(ConfigError::invalid_value(
                "endpoint",
                "TLS is enabled but endpoint scheme is not https",
            ));
        }

        // Validate timeouts
        if let Some(timeout) = self.timeout
            && timeout.is_zero()
        {
            return Err(ConfigError::invalid_value(
                "timeout",
                "Timeout must be greater than zero",
            ));
        }

        if let Some(connect_timeout) = self.connect_timeout
            && connect_timeout.is_zero()
        {
            return Err(ConfigError::invalid_value(
                "connect_timeout",
                "Connect timeout must be greater than zero",
            ));
        }

        // Validate message sizes
        if let Some(max_request_size) = self.max_request_size
            && max_request_size == 0
        {
            return Err(ConfigError::invalid_value(
                "max_request_size",
                "Max request size must be greater than zero",
            ));
        }

        if let Some(max_response_size) = self.max_response_size
            && max_response_size == 0
        {
            return Err(ConfigError::invalid_value(
                "max_response_size",
                "Max response size must be greater than zero",
            ));
        }

        // Validate concurrent requests
        if let Some(max_concurrent) = self.max_concurrent_requests
            && max_concurrent == 0
        {
            return Err(ConfigError::invalid_value(
                "max_concurrent_requests",
                "Max concurrent requests must be greater than zero",
            ));
        }

        Ok(())
    }
}

impl EuroraConfig {
    /// Create a new gRPC configuration with the given endpoint.
    pub fn new(endpoint: Url) -> Self {
        let use_tls = endpoint.scheme() == "https";
        Self {
            endpoint,
            use_tls,
            ..Default::default()
        }
    }

    /// Set the authentication token.
    pub fn with_auth_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Enable TLS with optional domain name.
    pub fn with_tls(mut self, domain: Option<String>) -> Self {
        self.use_tls = true;
        self.tls_domain = domain;
        self
    }

    /// Set the maximum request size.
    pub fn with_max_request_size(mut self, size: usize) -> Self {
        self.max_request_size = Some(size);
        self
    }

    /// Set the maximum response size.
    pub fn with_max_response_size(mut self, size: usize) -> Self {
        self.max_response_size = Some(size);
        self
    }

    /// Set the connection timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set keep-alive configuration.
    pub fn with_keep_alive(
        mut self,
        interval: Duration,
        timeout: Duration,
        while_idle: bool,
    ) -> Self {
        self.keep_alive_interval = Some(interval);
        self.keep_alive_timeout = Some(timeout);
        self.keep_alive_while_idle = while_idle;
        self
    }

    /// Set the maximum number of concurrent requests.
    pub fn with_max_concurrent_requests(mut self, max: usize) -> Self {
        self.max_concurrent_requests = Some(max);
        self
    }

    /// Set the user agent string.
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }
}
