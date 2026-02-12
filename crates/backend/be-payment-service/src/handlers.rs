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
use crate::webhook;

async fn resolve_customer_id(state: &AppState, email: &str) -> Result<String, PaymentError> {
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

pub async fn create_checkout_session(
    State(state): State<Arc<AppState>>,
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

pub async fn create_portal_session(
    State(state): State<Arc<AppState>>,
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

pub async fn get_subscription_status(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<SubscriptionStatus>, PaymentError> {
    let customer_id = match resolve_customer_id(&state, &claims.email).await {
        Ok(id) => id,
        Err(PaymentError::InvalidField(_)) => return Ok(Json(SubscriptionStatus::default())),
        Err(e) => return Err(e),
    };

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

pub async fn get_checkout_status(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    axum::extract::Query(params): axum::extract::Query<CheckoutStatusQuery>,
) -> Result<Json<CheckoutStatusResponse>, PaymentError> {
    let session = RetrieveCheckoutSession::new(params.session_id.as_str())
        .send(&state.client)
        .await?;

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
            let customer_id = session.customer.as_ref().map(|c| c.id().to_string());
            let subscription_id = session.subscription.as_ref().map(|s| s.id().to_string());
            let customer_email = session.customer_email.clone();

            info!(
                session_id = %session.id,
                customer = ?customer_id,
                "Checkout session completed"
            );

            if let Err(e) = webhook::on_checkout_completed(
                &state.db,
                customer_id,
                subscription_id,
                customer_email,
            )
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

            if let Err(e) =
                webhook::on_subscription_updated(&state.db, sub.id.to_string(), customer_id, status)
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

            if let Err(e) =
                webhook::on_subscription_deleted(&state.db, sub.id.to_string(), customer_id).await
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
