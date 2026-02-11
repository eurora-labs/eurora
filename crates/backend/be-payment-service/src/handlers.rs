use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use stripe_checkout::CheckoutSessionMode;
use stripe_checkout::checkout_session::{
    CreateCheckoutSession, CreateCheckoutSessionLineItems, RetrieveCheckoutSession,
};
use stripe_core::customer::ListCustomer;
use stripe_webhook::{Event, EventObject, Webhook};
use tracing::{error, info, warn};

use crate::auth::AuthUser;
use crate::error::PaymentError;
use crate::service::AppState;
use crate::types::{
    CheckoutStatusResponse, CreateCheckoutRequest, CreateCheckoutResponse, CreatePortalResponse,
    SubscriptionStatus,
};
use crate::webhook::WebhookEventHandler;

// ---------------------------------------------------------------------------
// Helper: resolve the authenticated user's Stripe customer by email
// ---------------------------------------------------------------------------

/// Looks up the Stripe customer that matches the authenticated user's email.
///
/// Returns the customer ID, or `Err(Unauthorized)` if no Stripe customer
/// exists yet for this email.
async fn resolve_customer_id<H: WebhookEventHandler>(
    state: &AppState<H>,
    email: &str,
) -> Result<String, PaymentError> {
    let page = ListCustomer::new()
        .email(email)
        .limit(1)
        .send(&state.client)
        .await?;

    page.data
        .first()
        .map(|c| c.id.to_string())
        .ok_or_else(|| PaymentError::InvalidField("no Stripe customer found for this account"))
}

// ---------------------------------------------------------------------------
// POST /payment/checkout
// ---------------------------------------------------------------------------

/// Creates a Stripe Checkout Session for a subscription and returns the URL.
///
/// The customer email is taken from the authenticated user's JWT claims.
/// If a Stripe customer already exists for that email, the existing customer
/// is reused to prevent duplicates.
pub async fn create_checkout_session<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateCheckoutRequest>,
) -> Result<Json<CreateCheckoutResponse>, PaymentError> {
    if !state
        .config
        .allowed_price_ids()
        .contains(&body.price_id.as_str())
    {
        return Err(PaymentError::InvalidField(
            "price_id is not a recognised plan",
        ));
    }

    let email = &claims.email;

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

    // Look up an existing Stripe customer by email to prevent duplicates.
    let existing = ListCustomer::new()
        .email(email)
        .limit(1)
        .send(&state.client)
        .await?;

    if let Some(customer) = existing.data.first() {
        info!(customer_id = %customer.id, %email, "Reusing existing Stripe customer");
        req = req.customer(&customer.id);
    } else {
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
///
/// The Stripe customer is resolved from the authenticated user's email — the
/// client never supplies a customer ID directly.
pub async fn create_portal_session<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<CreatePortalResponse>, PaymentError> {
    let customer_id = resolve_customer_id(&state, &claims.email).await?;
    let return_url = format!("{}/settings/billing", state.config.frontend_url);

    let session = stripe_billing::billing_portal_session::CreateBillingPortalSession::new()
        .customer(&customer_id)
        .return_url(&return_url)
        .send(&state.client)
        .await?;

    Ok(Json(CreatePortalResponse { url: session.url }))
}

// ---------------------------------------------------------------------------
// GET /payment/subscription?customer_id=cus_xxx
// ---------------------------------------------------------------------------

/// Returns the subscription status for the authenticated user.
///
/// The Stripe customer is resolved from the authenticated user's email — the
/// client never supplies a customer ID directly.
pub async fn get_subscription_status<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<SubscriptionStatus>, PaymentError> {
    let customer_id = resolve_customer_id(&state, &claims.email).await?;

    let page = stripe_billing::subscription::ListSubscription::new()
        .customer(&customer_id)
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

// ---------------------------------------------------------------------------
// GET /payment/checkout-status?session_id=cs_xxx
// ---------------------------------------------------------------------------

/// Verifies a checkout session's payment status so the frontend can confirm
/// that the payment actually went through before showing a success message.
///
/// The session's `customer_email` must match the authenticated user's email,
/// preventing users from probing arbitrary session IDs.
pub async fn get_checkout_status<H: WebhookEventHandler>(
    State(state): State<Arc<AppState<H>>>,
    AuthUser(claims): AuthUser,
    axum::extract::Query(params): axum::extract::Query<CheckoutStatusQuery>,
) -> Result<Json<CheckoutStatusResponse>, PaymentError> {
    let session = RetrieveCheckoutSession::new(params.session_id.as_str())
        .send(&state.client)
        .await?;

    // Ensure the checkout session belongs to the authenticated user.
    let session_email = session.customer_email.as_deref().unwrap_or_default();
    if !session_email.eq_ignore_ascii_case(&claims.email) {
        return Err(PaymentError::Unauthorized(
            "Session does not belong to this user".to_string(),
        ));
    }

    let status = session
        .status
        .map(|s| s.as_str().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());

    Ok(Json(CheckoutStatusResponse { status }))
}

#[derive(Debug, serde::Deserialize)]
pub struct CheckoutStatusQuery {
    pub session_id: String,
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
    use be_auth_core::JwtConfig;
    use tower::ServiceExt;

    use super::*;
    use crate::config::PaymentConfig;

    fn test_state() -> Arc<AppState> {
        // These env vars are required by JwtConfig::default().
        // Set them if not already present so tests can run standalone.
        // SAFETY: Tests run sequentially within this module; no concurrent reads.
        unsafe {
            if std::env::var("JWT_ACCESS_SECRET").is_err() {
                std::env::set_var("JWT_ACCESS_SECRET", "test_access_secret");
            }
            if std::env::var("JWT_REFRESH_SECRET").is_err() {
                std::env::set_var("JWT_REFRESH_SECRET", "test_refresh_secret");
            }
        }

        let config = PaymentConfig {
            stripe_secret_key: "sk_test_fake".to_string(),
            stripe_webhook_secret: "whsec_test_secret".to_string(),
            frontend_url: "http://localhost:5173".to_string(),
            pro_price_id: "price_pro".to_string(),
            enterprise_price_id: "price_enterprise".to_string(),
        };
        let client = stripe::Client::new(&config.stripe_secret_key);
        let jwt_config = Arc::new(JwtConfig::default());
        Arc::new(AppState {
            client,
            config,
            webhook_handler: Arc::new(crate::webhook::LoggingWebhookHandler),
            jwt_config,
        })
    }

    /// Helper: create a valid JWT access token for tests.
    fn test_access_token(_state: &Arc<AppState>) -> String {
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

        let secret =
            std::env::var("JWT_ACCESS_SECRET").unwrap_or_else(|_| "test_access_secret".into());
        let now = chrono::Utc::now().timestamp();
        let claims = auth_core::Claims {
            sub: "user_123".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            exp: now + 3600,
            iat: now,
            token_type: "access".to_string(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("failed to encode test JWT")
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
    async fn checkout_rejects_unauthenticated() {
        let state = test_state();
        let app = crate::create_router(state);

        let body = serde_json::json!({
            "price_id": "price_pro",
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payment/checkout")
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "127.0.0.1")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn checkout_rejects_unknown_price_id() {
        let state = test_state();
        let token = test_access_token(&state);
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
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-forwarded-for", "127.0.0.1")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
