use std::sync::Arc;

use stripe::{ClientBuilder, RequestStrategy};

use crate::config::PaymentConfig;
use crate::webhook::WebhookEventHandler;

/// Shared application state for the payment service.
pub struct AppState<H: WebhookEventHandler = crate::webhook::LoggingWebhookHandler> {
    /// The async-stripe HTTP client.
    pub client: stripe::Client,
    /// Payment-related configuration.
    pub config: PaymentConfig,
    /// Webhook event handler for provisioning / revoking access.
    pub webhook_handler: Arc<H>,
}

impl AppState {
    /// Creates a new `AppState` from environment variables with the default logging-only handler.
    pub fn from_env() -> Result<Self, crate::error::PaymentError> {
        Self::from_env_with_handler(Arc::new(crate::webhook::LoggingWebhookHandler))
    }
}

impl<H: WebhookEventHandler> AppState<H> {
    /// Creates a new `AppState` from environment variables with a custom webhook handler.
    pub fn from_env_with_handler(
        webhook_handler: Arc<H>,
    ) -> Result<Self, crate::error::PaymentError> {
        let config = PaymentConfig::from_env()?;
        let client = ClientBuilder::new(&config.stripe_secret_key)
            .request_strategy(RequestStrategy::ExponentialBackoff(3))
            .build()
            .map_err(|e| {
                crate::error::PaymentError::Config(format!("Failed to build Stripe client: {e}"))
            })?;
        Ok(Self {
            client,
            config,
            webhook_handler,
        })
    }
}
