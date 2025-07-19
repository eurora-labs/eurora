use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Request/Response types for API endpoints

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCustomerRequest {
    pub email: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomerResponse {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: Option<String>,
    pub active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    pub created: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePriceRequest {
    pub product_id: String,
    pub unit_amount: i64,
    pub currency: String,
    pub recurring: Option<RecurringRequest>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecurringRequest {
    pub interval: String, // day, week, month, year
    pub interval_count: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceResponse {
    pub id: String,
    pub product_id: String,
    pub unit_amount: i64,
    pub currency: String,
    pub recurring: Option<RecurringResponse>,
    pub active: bool,
    pub created: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecurringResponse {
    pub interval: String,
    pub interval_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub customer_id: String,
    pub price_id: String,
    pub quantity: Option<i32>,
    pub trial_period_days: Option<i32>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    pub id: String,
    pub customer_id: String,
    pub status: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub trial_start: Option<DateTime<Utc>>,
    pub trial_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub created: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePaymentIntentRequest {
    pub amount: i64,
    pub currency: String,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub automatic_payment_methods: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentIntentResponse {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub client_secret: String,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub created: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCheckoutSessionRequest {
    pub customer_id: Option<String>,
    pub customer_email: Option<String>,
    pub line_items: Vec<CheckoutLineItem>,
    pub mode: String, // payment, setup, subscription
    pub success_url: String,
    pub cancel_url: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutLineItem {
    pub price_id: String,
    pub quantity: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutSessionResponse {
    pub id: String,
    pub url: Option<String>,
    pub customer_id: Option<String>,
    pub mode: String,
    pub status: String,
    pub success_url: String,
    pub cancel_url: String,
    pub created: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookEventRequest {
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
    pub r#type: String,
}

// Internal types for database operations
#[derive(Debug, Clone)]
pub struct CustomerData {
    pub user_id: Uuid,
    pub stripe_customer_id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProductData {
    pub id: Uuid,
    pub stripe_product_id: String,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct SubscriptionData {
    pub id: Uuid,
    pub stripe_subscription_id: String,
    pub user_id: Uuid,
    pub status: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
}
