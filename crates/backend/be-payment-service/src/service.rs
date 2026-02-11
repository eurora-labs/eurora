use crate::config::PaymentConfig;

/// Shared application state for the payment service.
pub struct AppState {
    /// The async-stripe HTTP client.
    pub client: stripe::Client,
    /// Payment-related configuration.
    pub config: PaymentConfig,
}

impl AppState {
    /// Creates a new `AppState` from environment variables.
    pub fn from_env() -> Result<Self, crate::error::PaymentError> {
        let config = PaymentConfig::from_env()?;
        let client = stripe::Client::new(&config.stripe_secret_key);
        Ok(Self { client, config })
    }
}
