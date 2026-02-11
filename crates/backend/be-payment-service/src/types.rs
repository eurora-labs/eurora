use serde::{Deserialize, Serialize};

/// Request body for creating a checkout session.
#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    /// Stripe price ID to check out.
    pub price_id: String,
    /// Existing Stripe customer ID (optional).
    pub customer_id: Option<String>,
    /// Customer email used when no customer_id is provided.
    pub customer_email: Option<String>,
}

/// Response returned after creating a checkout session.
#[derive(Debug, Serialize)]
pub struct CreateCheckoutResponse {
    /// The checkout session ID.
    pub session_id: String,
    /// The URL to redirect the customer to.
    pub url: String,
}

/// Request body for creating a billing portal session.
#[derive(Debug, Deserialize)]
pub struct CreatePortalRequest {
    /// The Stripe customer ID.
    pub customer_id: String,
}

/// Response returned after creating a billing portal session.
#[derive(Debug, Serialize)]
pub struct CreatePortalResponse {
    /// The URL to redirect the customer to.
    pub url: String,
}

/// Lightweight subscription status returned to the frontend.
#[derive(Debug, Serialize)]
pub struct SubscriptionStatus {
    pub subscription_id: Option<String>,
    pub status: Option<String>,
    pub price_id: Option<String>,
    pub cancel_at: Option<i64>,
    pub cancel_at_period_end: Option<bool>,
}
