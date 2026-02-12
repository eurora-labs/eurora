use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub price_id: String,
}

#[derive(Debug, Serialize)]
pub struct CreateCheckoutResponse {
    pub session_id: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct CreatePortalResponse {
    pub url: String,
}

#[derive(Debug, Default, Serialize)]
pub struct SubscriptionStatus {
    pub subscription_id: Option<String>,
    pub status: Option<String>,
    pub price_id: Option<String>,
    pub cancel_at: Option<i64>,
    pub cancel_at_period_end: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutStatusResponse {
    pub status: String,
}
