use std::sync::Arc;

use be_auth_core::JwtConfig;
use be_remote_db::DatabaseManager;
use stripe::{ClientBuilder, RequestStrategy};

use crate::config::PaymentConfig;

pub struct AppState {
    pub client: stripe::Client,
    pub config: PaymentConfig,
    pub db: Arc<DatabaseManager>,
    pub jwt_config: Arc<JwtConfig>,
}

impl AppState {
    pub fn from_env(db: Arc<DatabaseManager>) -> Result<Self, crate::error::PaymentError> {
        let config = PaymentConfig::from_env()?;
        let client = ClientBuilder::new(&config.stripe_secret_key)
            .request_strategy(RequestStrategy::ExponentialBackoff(3))
            .build()
            .map_err(|e| {
                crate::error::PaymentError::Config(format!("Failed to build Stripe client: {e}"))
            })?;
        let jwt_config = Arc::new(JwtConfig::default());
        Ok(Self {
            client,
            config,
            db,
            jwt_config,
        })
    }
}
