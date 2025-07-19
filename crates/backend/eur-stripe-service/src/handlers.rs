use crate::{error::Result, service::StripeService, types::*};
use axum::{
    extract::{Path, State},
    response::Json,
};
use chrono::Utc;

// Health check endpoint
pub async fn health() -> Result<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

// Customer endpoints
pub async fn create_customer(
    State(service): State<StripeService>,
    Json(request): Json<CreateCustomerRequest>,
) -> Result<Json<CustomerResponse>> {
    let customer = service.create_customer(request).await?;
    Ok(Json(customer))
}

pub async fn get_customer(
    State(service): State<StripeService>,
    Path(customer_id): Path<String>,
) -> Result<Json<CustomerResponse>> {
    let customer = service.get_customer(&customer_id).await?;
    Ok(Json(customer))
}

// Product endpoints
pub async fn create_product(
    State(service): State<StripeService>,
    Json(request): Json<CreateProductRequest>,
) -> Result<Json<ProductResponse>> {
    let product = service.create_product(request).await?;
    Ok(Json(product))
}

pub async fn get_product(
    State(service): State<StripeService>,
    Path(product_id): Path<String>,
) -> Result<Json<ProductResponse>> {
    let product = service.get_product(&product_id).await?;
    Ok(Json(product))
}

// Price endpoints
pub async fn create_price(
    State(service): State<StripeService>,
    Json(request): Json<CreatePriceRequest>,
) -> Result<Json<PriceResponse>> {
    let price = service.create_price(request).await?;
    Ok(Json(price))
}

// Subscription endpoints
pub async fn create_subscription(
    State(service): State<StripeService>,
    Json(request): Json<CreateSubscriptionRequest>,
) -> Result<Json<SubscriptionResponse>> {
    let subscription = service.create_subscription(request).await?;
    Ok(Json(subscription))
}

// Payment Intent endpoints
pub async fn create_payment_intent(
    State(service): State<StripeService>,
    Json(request): Json<CreatePaymentIntentRequest>,
) -> Result<Json<PaymentIntentResponse>> {
    let payment_intent = service.create_payment_intent(request).await?;
    Ok(Json(payment_intent))
}

// Checkout Session endpoints
pub async fn create_checkout_session(
    State(service): State<StripeService>,
    Json(request): Json<CreateCheckoutSessionRequest>,
) -> Result<Json<CheckoutSessionResponse>> {
    let session = service.create_checkout_session(request).await?;
    Ok(Json(session))
}
