use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub stripe_customer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordCredentials {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub email_verified: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePasswordRequest {
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthCredentials {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub access_token: Option<Vec<u8>>,
    pub refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOAuthCredentialsRequest {
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub access_token: Option<Vec<u8>>,
    pub refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRefreshTokenRequest {
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOAuthCredentialsRequest {
    pub access_token: Option<Vec<u8>>,
    pub refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthState {
    pub id: Uuid,
    pub state: String,
    pub pkce_verifier: String,
    pub redirect_uri: String,
    pub ip_address: Option<ipnet::IpNet>,
    pub consumed: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOAuthStateRequest {
    pub state: String,
    pub pkce_verifier: String,
    pub redirect_uri: String,
    pub ip_address: Option<ipnet::IpNet>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LoginToken {
    pub id: Uuid,
    pub token: String,
    pub consumed: bool,
    pub expires_at: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLoginTokenRequest {
    pub token: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLoginTokenRequest {
    pub user_id: Uuid,
}

// Stripe-related types
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: Uuid,
    pub stripe_product_id: String,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub stripe_product_id: String,
    pub name: String,
    pub description: Option<String>,
    pub active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Price {
    pub id: Uuid,
    pub stripe_price_id: String,
    pub product_id: Uuid,
    pub active: bool,
    pub currency: String,
    pub unit_amount: Option<i64>,
    pub recurring_interval: Option<String>,
    pub recurring_interval_count: Option<i32>,
    pub billing_scheme: String,
    pub tiers: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePriceRequest {
    pub stripe_price_id: String,
    pub product_id: Uuid,
    pub active: Option<bool>,
    pub currency: String,
    pub unit_amount: Option<i64>,
    pub recurring_interval: Option<String>,
    pub recurring_interval_count: Option<i32>,
    pub billing_scheme: Option<String>,
    pub tiers: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePriceRequest {
    pub active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub stripe_subscription_id: String,
    pub user_id: Uuid,
    pub status: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub trial_start: Option<DateTime<Utc>>,
    pub trial_end: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub stripe_subscription_id: String,
    pub user_id: Uuid,
    pub status: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub cancel_at_period_end: Option<bool>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub trial_start: Option<DateTime<Utc>>,
    pub trial_end: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubscriptionRequest {
    pub status: Option<String>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: Option<bool>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub trial_start: Option<DateTime<Utc>>,
    pub trial_end: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SubscriptionItem {
    pub id: Uuid,
    pub stripe_subscription_item_id: String,
    pub subscription_id: Uuid,
    pub price_id: Uuid,
    pub quantity: i32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionItemRequest {
    pub stripe_subscription_item_id: String,
    pub subscription_id: Uuid,
    pub price_id: Uuid,
    pub quantity: Option<i32>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubscriptionItemRequest {
    pub quantity: Option<i32>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    pub id: Uuid,
    pub stripe_invoice_id: String,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub status: String,
    pub currency: String,
    pub amount_due: i64,
    pub amount_paid: i64,
    pub amount_remaining: i64,
    pub subtotal: i64,
    pub total: i64,
    pub tax: i64,
    pub hosted_invoice_url: Option<String>,
    pub invoice_pdf: Option<String>,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub due_date: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceRequest {
    pub stripe_invoice_id: String,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub status: String,
    pub currency: String,
    pub amount_due: i64,
    pub amount_paid: Option<i64>,
    pub amount_remaining: Option<i64>,
    pub subtotal: i64,
    pub total: i64,
    pub tax: Option<i64>,
    pub hosted_invoice_url: Option<String>,
    pub invoice_pdf: Option<String>,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub due_date: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInvoiceRequest {
    pub status: Option<String>,
    pub amount_paid: Option<i64>,
    pub amount_remaining: Option<i64>,
    pub paid_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub stripe_payment_intent_id: String,
    pub user_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub payment_method_type: Option<String>,
    pub payment_method_id: Option<String>,
    pub client_secret: Option<String>,
    pub confirmation_method: Option<String>,
    pub receipt_email: Option<String>,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentRequest {
    pub stripe_payment_intent_id: String,
    pub user_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub payment_method_type: Option<String>,
    pub payment_method_id: Option<String>,
    pub client_secret: Option<String>,
    pub confirmation_method: Option<String>,
    pub receipt_email: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePaymentRequest {
    pub status: Option<String>,
    pub payment_method_type: Option<String>,
    pub payment_method_id: Option<String>,
    pub receipt_email: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CheckoutSession {
    pub id: Uuid,
    pub stripe_checkout_session_id: String,
    pub user_id: Option<Uuid>,
    pub mode: String,
    pub status: String,
    pub currency: Option<String>,
    pub amount_total: Option<i64>,
    pub customer_email: Option<String>,
    pub success_url: String,
    pub cancel_url: String,
    pub payment_intent_id: Option<String>,
    pub subscription_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckoutSessionRequest {
    pub stripe_checkout_session_id: String,
    pub user_id: Option<Uuid>,
    pub mode: String,
    pub status: String,
    pub currency: Option<String>,
    pub amount_total: Option<i64>,
    pub customer_email: Option<String>,
    pub success_url: String,
    pub cancel_url: String,
    pub payment_intent_id: Option<String>,
    pub subscription_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckoutSessionRequest {
    pub status: Option<String>,
    pub payment_intent_id: Option<String>,
    pub subscription_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub stripe_event_id: String,
    pub event_type: String,
    pub api_version: Option<String>,
    pub data: serde_json::Value,
    pub processed: bool,
    pub processed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookEventRequest {
    pub stripe_event_id: String,
    pub event_type: String,
    pub api_version: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWebhookEventRequest {
    pub processed: Option<bool>,
    pub processed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserStripeCustomerRequest {
    pub stripe_customer_id: String,
}
