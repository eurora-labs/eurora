/// Payment service configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct PaymentConfig {
    /// Stripe secret key (sk_test_... or sk_live_...).
    pub stripe_secret_key: String,
    /// Stripe webhook signing secret (whsec_...).
    pub stripe_webhook_secret: String,
    /// Frontend URL used for checkout session redirect URLs.
    pub frontend_url: String,
    /// Stripe price ID for the Pro plan.
    pub pro_price_id: String,
    /// Stripe price ID for the Enterprise plan.
    pub enterprise_price_id: String,
}

impl PaymentConfig {
    /// Loads configuration from environment variables.
    ///
    /// Required env vars:
    /// - `STRIPE_SECRET_KEY`
    /// - `STRIPE_WEBHOOK_SECRET`
    ///
    /// Optional env vars (with defaults):
    /// - `FRONTEND_URL` (default: `http://localhost:5173`)
    /// - `STRIPE_PRO_PRICE_ID` (default: empty string)
    /// - `STRIPE_ENTERPRISE_PRICE_ID` (default: empty string)
    pub fn from_env() -> Result<Self, crate::error::PaymentError> {
        let stripe_secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(|_| {
            crate::error::PaymentError::Config(
                "STRIPE_SECRET_KEY environment variable must be set".into(),
            )
        })?;

        let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").map_err(|_| {
            crate::error::PaymentError::Config(
                "STRIPE_WEBHOOK_SECRET environment variable must be set".into(),
            )
        })?;

        let frontend_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

        let pro_price_id = std::env::var("STRIPE_PRO_PRICE_ID").unwrap_or_default();

        let enterprise_price_id = std::env::var("STRIPE_ENTERPRISE_PRICE_ID").unwrap_or_default();

        Ok(Self {
            stripe_secret_key,
            stripe_webhook_secret,
            frontend_url,
            pro_price_id,
            enterprise_price_id,
        })
    }
}
