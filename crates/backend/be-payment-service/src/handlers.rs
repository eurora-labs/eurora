use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use stripe_checkout::CheckoutSessionMode;
use stripe_checkout::checkout_session::{CreateCheckoutSession, CreateCheckoutSessionLineItems};
use stripe_core::customer::ListCustomer;
use stripe_webhook::{Event, EventObject, Webhook};
use tracing::{error, info, warn};

use crate::error::PaymentError;
use crate::service::AppState;
use crate::types::{
    CreateCheckoutRequest, CreateCheckoutResponse, CreatePortalRequest, CreatePortalResponse,
    SubscriptionStatus,
};
use crate::webhook::WebhookEventHandler;

// ---------------------------------------------------------------------------
// POST /payment/checkout
// ---------------------------------------------------------------------------

/// Creates a Stripe Checkout Session for a subscription and returns the URL.
pub async fn create_checkout_session<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
    Json(body): Json<CreateCheckoutRequest>,
) -> Result<Json<CreateCheckoutResponse>, PaymentError> {
    if !state
        .config
        .allowed_price_ids()
        .contains(&body.price_id.as_str())
    {
        return Err(PaymentError::MissingField(
            "price_id is not a recognised plan",
        ));
    }

    let line_items = vec![CreateCheckoutSessionLineItems {
        quantity: Some(1),
        price: Some(body.price_id),
        ..Default::default()
    }];

    // Double braces escape in format! to produce the literal {CHECKOUT_SESSION_ID}
    // that Stripe replaces with the actual session ID on redirect.
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

    let url = session
        .url
        .ok_or_else(|| PaymentError::MissingField("checkout session URL"))?;

    Ok(Json(CreateCheckoutResponse {
        session_id: session.id.to_string(),
        url,
    }))
}

// ---------------------------------------------------------------------------
// POST /payment/portal
// ---------------------------------------------------------------------------

/// Creates a Stripe Billing Portal session so customers can manage their subscription.
pub async fn create_portal_session<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
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
pub async fn get_subscription_status<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
    axum::extract::Query(params): axum::extract::Query<CustomerQuery>,
) -> Result<Json<SubscriptionStatus>, PaymentError> {
    let page = stripe_billing::subscription::ListSubscription::new()
        .customer(&params.customer_id)
        .limit(1)
        .send(&state.client)
        .await?;

    let status = page.data.first().map(|sub| SubscriptionStatus {
        subscription_id: Some(sub.id.to_string()),
        status: Some(sub.status.to_string()),
        price_id: sub.items.data.first().map(|i| i.price.id.to_string()),
        cancel_at: sub.cancel_at,
        cancel_at_period_end: Some(sub.cancel_at_period_end),
    });

    Ok(Json(status.unwrap_or_default()))
}

#[derive(Debug, serde::Deserialize)]
pub struct CustomerQuery {
    pub customer_id: String,
}

// ---------------------------------------------------------------------------
// GET /payment/customers?limit=20&starting_after=cus_xxx
// ---------------------------------------------------------------------------

const DEFAULT_CUSTOMER_PAGE_SIZE: i64 = 20;

/// Lists Stripe customers with pagination (admin/debug endpoint).
pub async fn list_customers<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
    axum::extract::Query(params): axum::extract::Query<CustomerListQuery>,
) -> Result<impl IntoResponse, PaymentError> {
    let limit = params.limit.unwrap_or(DEFAULT_CUSTOMER_PAGE_SIZE).min(100);

    let mut req = ListCustomer::new().limit(limit);
    if let Some(ref cursor) = params.starting_after {
        req = req.starting_after(cursor);
    }

    let page = req.send(&state.client).await?;

    let summary: Vec<serde_json::Value> = page
        .data
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "email": c.email,
                "name": c.name,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "data": summary,
        "has_more": page.has_more,
    })))
}

#[derive(Debug, serde::Deserialize)]
pub struct CustomerListQuery {
    pub limit: Option<i64>,
    pub starting_after: Option<String>,
}

// ---------------------------------------------------------------------------
// POST /payment/webhook
// ---------------------------------------------------------------------------

/// Handles incoming Stripe webhook events with signature verification.
pub async fn handle_webhook<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
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
            let customer_id = session.customer.as_ref().map(|c| c.id().to_string());
            let subscription_id = session.subscription.as_ref().map(|s| s.id().to_string());
            let customer_email = session.customer_email.clone();

            info!(
                session_id = %session.id,
                customer = ?customer_id,
                "Checkout session completed"
            );

            if let Err(e) = state
                .webhook_handler
                .on_checkout_completed(customer_id, subscription_id, customer_email)
                .await
            {
                error!(error = %e, "Failed to provision access after checkout");
                return Err(e);
            }
        }
        EventObject::CustomerSubscriptionUpdated(sub) => {
            let customer_id = Some(sub.customer.id().to_string());
            let status = sub.status.to_string();

            info!(
                subscription_id = %sub.id,
                status = %status,
                "Subscription updated"
            );

            if let Err(e) = state
                .webhook_handler
                .on_subscription_updated(sub.id.to_string(), customer_id, status)
                .await
            {
                error!(error = %e, "Failed to handle subscription update");
                return Err(e);
            }
        }
        EventObject::CustomerSubscriptionDeleted(sub) => {
            let customer_id = Some(sub.customer.id().to_string());

            info!(
                subscription_id = %sub.id,
                "Subscription deleted"
            );

            if let Err(e) = state
                .webhook_handler
                .on_subscription_deleted(sub.id.to_string(), customer_id)
                .await
            {
                error!(error = %e, "Failed to revoke access after subscription deletion");
                return Err(e);
            }
        }
        _ => {
            warn!(event_type = %event.type_, "Unhandled webhook event");
        }
    }

    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;
    use crate::config::PaymentConfig;

    fn test_state() -> Arc<AppState> {
        let config = PaymentConfig {
            stripe_secret_key: "sk_test_fake".to_string(),
            stripe_webhook_secret: "whsec_test_secret".to_string(),
            frontend_url: "http://localhost:5173".to_string(),
            pro_price_id: "price_pro".to_string(),
            enterprise_price_id: "price_enterprise".to_string(),
        };
        let client = stripe::Client::new(&config.stripe_secret_key);
        Arc::new(AppState {
            client,
            config,
            webhook_handler: Arc::new(crate::webhook::LoggingWebhookHandler),
        })
    }

    #[tokio::test]
    async fn webhook_rejects_missing_signature() {
        let state = test_state();
        let app = crate::create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payment/webhook")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn webhook_rejects_bad_signature() {
        let state = test_state();
        let app = crate::create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payment/webhook")
                    .header("content-type", "application/json")
                    .header("stripe-signature", "t=123,v1=badsig")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn webhook_accepts_valid_signature() {
        let state = test_state();
        let secret = state.config.stripe_webhook_secret.clone();
        let app = crate::create_router(state);

        let payload = serde_json::json!({
            "id": "evt_test",
            "object": "event",
            "api_version": "2017-05-25",
            "created": 1533204620,
            "livemode": false,
            "pending_webhooks": 1,
            "data": {
                "object": {
                    "object": "bank_account",
                    "country": "us",
                    "currency": "usd",
                    "id": "ba_test",
                    "last4": "6789",
                    "status": "verified"
                }
            },
            "type": "account.external_account.created"
        })
        .to_string();

        let sig = Webhook::generate_test_header(&payload, &secret, None);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payment/webhook")
                    .header("content-type", "application/json")
                    .header("stripe-signature", sig)
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn checkout_rejects_unknown_price_id() {
        let state = test_state();
        let app = crate::create_router(state);

        let body = serde_json::json!({
            "price_id": "price_unknown_plan",
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payment/checkout")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
