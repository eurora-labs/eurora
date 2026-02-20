use axum::http::HeaderValue;

#[derive(Debug, Clone)]
pub struct PaymentConfig {
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub frontend_url: String,
    pub pro_price_id: String,
    pub approved_beta_emails: Vec<String>,
}

impl PaymentConfig {
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

        HeaderValue::from_str(&frontend_url).map_err(|e| {
            crate::error::PaymentError::Config(format!(
                "FRONTEND_URL '{frontend_url}' is not a valid header value: {e}"
            ))
        })?;

        let pro_price_id = std::env::var("STRIPE_PRO_PRICE_ID").map_err(|_| {
            crate::error::PaymentError::Config(
                "STRIPE_PRO_PRICE_ID environment variable must be set".into(),
            )
        })?;

        let approved_beta_emails = std::env::var("APPROVED_BETA_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_lowercase().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        Ok(Self {
            stripe_secret_key,
            stripe_webhook_secret,
            frontend_url,
            pro_price_id,
            approved_beta_emails,
        })
    }

    pub fn is_approved_beta_email(&self, email: &str) -> bool {
        let email = email.to_lowercase();
        self.approved_beta_emails
            .iter()
            .any(|approved| approved == "*" || *approved == email)
    }

    pub fn allowed_price_ids(&self) -> Vec<&str> {
        vec![&self.pro_price_id]
    }
}
