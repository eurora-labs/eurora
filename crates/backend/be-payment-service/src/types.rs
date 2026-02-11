use serde::{Deserialize, Serialize};

/// Request body for creating a checkout session.
///
/// The customer email is derived from the authenticated user's JWT claims,
/// and the Stripe customer ID is resolved server-side to prevent duplicates.
#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    /// Stripe price ID to check out.
    pub price_id: String,
}

/// Response returned after creating a checkout session.
#[derive(Debug, Serialize)]
pub struct CreateCheckoutResponse {
    /// The checkout session ID.
    pub session_id: String,
    /// The URL to redirect the customer to.
    pub url: String,
}

/// Response returned after creating a billing portal session.
#[derive(Debug, Serialize)]
pub struct CreatePortalResponse {
    /// The URL to redirect the customer to.
    pub url: String,
}

/// Lightweight subscription status returned to the frontend.
#[derive(Debug, Default, Serialize)]
pub struct SubscriptionStatus {
    pub subscription_id: Option<String>,
    pub status: Option<String>,
    pub price_id: Option<String>,
    pub cancel_at: Option<i64>,
    pub cancel_at_period_end: Option<bool>,
}

/// Response for verifying a checkout session's payment status.
#[derive(Debug, Serialize)]
pub struct CheckoutStatusResponse {
    /// The session status: "complete", "open", or "expired".
    pub status: String,
}
