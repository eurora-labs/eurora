use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub stripe: StripeConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConfig {
    pub secret_key: String,
    pub publishable_key: String,
    pub webhook_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, crate::error::StripeServiceError> {
        Ok(Config {
            server: ServerConfig {
                host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("PORT")
                    .unwrap_or_else(|_| "3003".to_string())
                    .parse()
                    .map_err(|_| {
                        crate::error::StripeServiceError::ConfigError(config::ConfigError::Message(
                            "Invalid PORT value".to_string(),
                        ))
                    })?,
            },
            stripe: StripeConfig {
                secret_key: env::var("STRIPE_SECRET_KEY")?,
                publishable_key: env::var("STRIPE_PUBLISHABLE_KEY")?,
                webhook_secret: env::var("STRIPE_WEBHOOK_SECRET")?,
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")?,
            },
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3003,
            },
            stripe: StripeConfig {
                secret_key: "sk_test_...".to_string(),
                publishable_key: "pk_test_...".to_string(),
                webhook_secret: "whsec_...".to_string(),
            },
            database: DatabaseConfig {
                url: "postgresql://localhost/eurora".to_string(),
            },
        }
    }
}
