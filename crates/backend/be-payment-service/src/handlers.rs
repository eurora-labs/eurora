use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use stripe_checkout::CheckoutSessionMode;
use stripe_checkout::checkout_session::{CreateCheckoutSession, CreateCheckoutSessionLineItems};
use stripe_core::customer::ListCustomer;
use stripe_webhook::{Event, EventObject, Webhook};
use tracing::{info, warn};

use crate::error::PaymentError;
use crate::service::AppState;
use crate::types::{
    CreateCheckoutRequest, CreateCheckoutResponse, CreatePortalRequest, CreatePortalResponse,
    SubscriptionStatus,
};

// ---------------------------------------------------------------------------
// POST /payment/checkout
// ---------------------------------------------------------------------------

/// Creates a Stripe Checkout Session for a subscription and returns the URL.
pub async fn create_checkout_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateCheckoutRequest>,
) -> Result<Json<CreateCheckoutResponse>, PaymentError> {
    let line_items = vec![CreateCheckoutSessionLineItems {
        quantity: Some(1),
        price: Some(body.price_id),
        ..Default::default()
    }];

    let success_url = format!(
        "{}/payment/thanks?session_id={{CHECKOUT_SESSION_ID}}",
        state.config.frontend_url
    );
    let cancel_url = format!("{}/pricing", state.config.frontend_url);

    let mut req = CreateCheckoutSession::new()
        .mode(CheckoutSessionMode::Subscription)
        .line_items(line_items)
        .success_url(&success_url)
        .cancel_url(&cancel_url);

    if let Some(ref customer_id) = body.customer_id {
        req = req.customer(customer_id);
    } else if let Some(ref email) = body.customer_email {
        req = req.customer_email(email);
    }

    let session = req.send(&state.client).await?;

    Ok(Json(CreateCheckoutResponse {
        session_id: session.id.to_string(),
        url: session.url.unwrap_or_default(),
    }))
}

// ---------------------------------------------------------------------------
// POST /payment/portal
// ---------------------------------------------------------------------------

/// Creates a Stripe Billing Portal session so customers can manage their subscription.
pub async fn create_portal_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreatePortalRequest>,
) -> Result<Json<CreatePortalResponse>, PaymentError> {
    let return_url = format!("{}/settings/billing", state.config.frontend_url);

    let session = stripe_billing::billing_portal_session::CreateBillingPortalSession::new()
        .customer(&body.customer_id)
        .return_url(&return_url)
        .send(&state.client)
        .await?;

    Ok(Json(CreatePortalResponse { url: session.url }))
}

// ---------------------------------------------------------------------------
// GET /payment/subscription?customer_id=cus_xxx
// ---------------------------------------------------------------------------

/// Returns the subscription status for a given customer.
pub async fn get_subscription_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<CustomerQuery>,
) -> Result<Json<SubscriptionStatus>, PaymentError> {
    use futures_util::TryStreamExt;

    let subscriptions = stripe_billing::subscription::ListSubscription::new()
        .customer(&params.customer_id)
        .paginate()
        .stream(&state.client)
        .try_collect::<Vec<_>>()
        .await?;

    let status = subscriptions.first().map(|sub| SubscriptionStatus {
        subscription_id: Some(sub.id.to_string()),
        status: Some(sub.status.to_string()),
        price_id: sub.items.data.first().map(|i| i.price.id.to_string()),
        cancel_at: sub.cancel_at,
        cancel_at_period_end: Some(sub.cancel_at_period_end),
    });

    Ok(Json(status.unwrap_or(SubscriptionStatus {
        subscription_id: None,
        status: None,
        price_id: None,
        cancel_at: None,
        cancel_at_period_end: None,
    })))
}

#[derive(Debug, serde::Deserialize)]
pub struct CustomerQuery {
    pub customer_id: String,
}

// ---------------------------------------------------------------------------
// GET /payment/customers
// ---------------------------------------------------------------------------

/// Lists all Stripe customers (admin/debug endpoint).
pub async fn list_customers(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, PaymentError> {
    use futures_util::TryStreamExt;

    let customers = ListCustomer::new()
        .paginate()
        .stream(&state.client)
        .try_collect::<Vec<_>>()
        .await?;

    let summary: Vec<serde_json::Value> = customers
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "email": c.email,
                "name": c.name,
            })
        })
        .collect();

    Ok(Json(summary))
}

// ---------------------------------------------------------------------------
// POST /payment/webhook
// ---------------------------------------------------------------------------

/// Handles incoming Stripe webhook events with signature verification.
pub async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, PaymentError> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(PaymentError::WebhookSignatureInvalid)?;

    let event: Event =
        Webhook::construct_event(&body, signature, &state.config.stripe_webhook_secret)
            .map_err(|_| PaymentError::WebhookSignatureInvalid)?;

    match event.data.object {
        EventObject::CheckoutSessionCompleted(session) => {
            info!(
                session_id = %session.id,
                customer = ?session.customer,
                "Checkout session completed"
            );
            // TODO: provision access / update user record in database
        }
        EventObject::CustomerSubscriptionUpdated(sub) => {
            info!(
                subscription_id = %sub.id,
                status = %sub.status,
                "Subscription updated"
            );
        }
        EventObject::CustomerSubscriptionDeleted(sub) => {
            info!(
                subscription_id = %sub.id,
                "Subscription deleted"
            );
            // TODO: revoke access
        }
        _ => {
            warn!(event_type = %event.type_, "Unhandled webhook event");
        }
    }

    Ok(StatusCode::OK)
}
