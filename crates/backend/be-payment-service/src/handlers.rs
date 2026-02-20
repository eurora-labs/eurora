use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use stripe_checkout::CheckoutSessionMode;
use stripe_checkout::checkout_session::{
    CreateCheckoutSession, CreateCheckoutSessionLineItems, RetrieveCheckoutSession,
};
use stripe_core::customer::{CreateCustomer, ListCustomer};
use stripe_webhook::{Event, EventObject, Webhook};
use tracing::{error, info, warn};

use crate::analytics;
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

    if let Some(c) = page.data.first() {
        return Ok(c.id.to_string());
    }

    let customer = CreateCustomer::new()
        .email(email)
        .send(&state.client)
        .await?;

    let customer_id = customer.id.to_string();

    let raw_data = serde_json::to_value(&customer).unwrap_or_default();
    let mut tx = state
        .db
        .pool
        .begin()
        .await
        .map_err(|e| anyhow::anyhow!("begin tx: {e}"))?;

    state
        .db
        .upsert_stripe_customer(&mut *tx, &customer_id, Some(email), &raw_data)
        .await
        .map_err(|e| anyhow::anyhow!("upsert stripe customer: {e}"))?;

    if let Ok(user) = state.db.get_user_by_email(email).await {
        state
            .db
            .ensure_account_for_user(&mut *tx, user.id, &customer_id)
            .await
            .map_err(|e| anyhow::anyhow!("link account to stripe customer: {e}"))?;
    }

    tx.commit()
        .await
        .map_err(|e| anyhow::anyhow!("commit tx: {e}"))?;

    info!(%customer_id, %email, "Auto-created Stripe customer for new account");

    Ok(customer_id)
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
        analytics::track_checkout_session_creation_failed(Some(&body.price_id), "invalid_price_id");
        return Err(PaymentError::InvalidField(
            "price_id is not a recognised plan",
        ));
    }

    let email = &claims.email;

    let line_items = vec![CreateCheckoutSessionLineItems {
        quantity: Some(1),
        price: Some(body.price_id.clone()),
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
        .await
        .inspect_err(|_e| {
            analytics::track_checkout_session_creation_failed(Some(&body.price_id), "stripe_error");
        })?;

    if let Some(customer) = existing.data.first() {
        info!(customer_id = %customer.id, %email, "Reusing existing Stripe customer");
        req = req.customer(&customer.id);
    } else {
        req = req.customer_email(email);
    }

    let session = req.send(&state.client).await.inspect_err(|_e| {
        analytics::track_checkout_session_creation_failed(Some(&body.price_id), "stripe_error");
    })?;

    let url = session.url.ok_or_else(|| {
        analytics::track_checkout_session_creation_failed(
            Some(&body.price_id),
            "missing_checkout_url",
        );
        PaymentError::MissingField("checkout session URL")
    })?;

    analytics::track_checkout_session_created(&body.price_id);

    Ok(Json(CreateCheckoutResponse {
        session_id: session.id.to_string(),
        url,
    }))
}

pub async fn create_portal_session(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<CreatePortalResponse>, PaymentError> {
    let customer_id = resolve_customer_id(&state, &claims.email)
        .await
        .inspect_err(|e| {
            analytics::track_billing_portal_failed(e.error_kind());
        })?;
    let return_url = format!("{}/settings/billing", state.config.frontend_url);

    let session = stripe_billing::billing_portal_session::CreateBillingPortalSession::new()
        .customer(&customer_id)
        .return_url(&return_url)
        .send(&state.client)
        .await
        .map_err(|e| {
            analytics::track_billing_portal_failed("stripe_error");
            PaymentError::from(e)
        })?;

    analytics::track_billing_portal_created();

    Ok(Json(CreatePortalResponse { url: session.url }))
}

pub async fn get_subscription_status(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<SubscriptionStatus>, PaymentError> {
    let customer_id = match resolve_customer_id(&state, &claims.email).await {
        Ok(id) => id,
        Err(PaymentError::InvalidField(_)) => {
            analytics::track_subscription_status_checked(None, None);
            return Ok(Json(SubscriptionStatus::default()));
        }
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

    let result = status.unwrap_or_default();
    analytics::track_subscription_status_checked(
        result.status.as_deref(),
        result.price_id.as_deref(),
    );

    Ok(Json(result))
}

pub async fn get_checkout_status(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    axum::extract::Query(params): axum::extract::Query<CheckoutStatusQuery>,
) -> Result<Json<CheckoutStatusResponse>, PaymentError> {
    let session = RetrieveCheckoutSession::new(params.session_id.as_str())
        .send(&state.client)
        .await?;

    let session_email = session
        .customer_email
        .as_deref()
        .or_else(|| {
            session
                .customer_details
                .as_ref()
                .and_then(|d| d.email.as_deref())
        })
        .unwrap_or_default();
    if !session_email.eq_ignore_ascii_case(&claims.email) {
        return Err(PaymentError::Unauthorized(
            "Session does not belong to this user".to_string(),
        ));
    }

    let status = session
        .status
        .map(|s| s.as_str().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());

    analytics::track_checkout_status_checked(&status);

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

    let event_id = event.id.as_str();
    let event_type = event.type_.as_str();

    // Atomic idempotency: try to claim the event before processing.
    // Returns false if the event was already recorded by a concurrent handler.
    if !state
        .db
        .try_claim_webhook_event(event_id, event_type)
        .await
        .unwrap_or(false)
    {
        info!(%event_id, %event_type, "Webhook event already processed — skipping");
        return Ok(StatusCode::OK);
    }

    let raw_data: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        error!(%event_id, error = %e, "Failed to parse webhook body as JSON");
        PaymentError::Internal(anyhow::anyhow!("webhook body JSON parse error: {e}"))
    })?;

    match event.data.object {
        EventObject::CheckoutSessionCompleted(session) => {
            let customer_id = session.customer.as_ref().map(|c| c.id().to_string());
            let subscription_id = session.subscription.as_ref().map(|s| s.id().to_string());
            let customer_email = session.customer_email.clone();

            // Try to get the expanded subscription object for period/item data
            let subscription_obj = session.subscription.as_ref().and_then(|s| s.as_object());

            info!(
                %event_id,
                session_id = %session.id,
                customer = ?customer_id,
                "Checkout session completed"
            );

            if let Err(e) = webhook::on_checkout_completed(
                &state.db,
                customer_id,
                subscription_id.clone(),
                customer_email,
                subscription_obj,
                &raw_data,
            )
            .await
            {
                error!(%event_id, error = %e, "Failed to provision access after checkout");
                return Err(e);
            }

            analytics::track_webhook_checkout_completed(subscription_id.is_some(), true);
        }
        EventObject::CustomerSubscriptionUpdated(sub) => {
            info!(
                %event_id,
                subscription_id = %sub.id,
                status = %sub.status,
                "Subscription updated"
            );

            if let Err(e) = webhook::on_subscription_updated(&state.db, &sub, &raw_data).await {
                error!(%event_id, error = %e, "Failed to handle subscription update");
                return Err(e);
            }

            let plan_id = webhook::resolve_plan_id_for_tracking(&state.db, &sub).await;
            analytics::track_webhook_subscription_updated(
                &sub.status.to_string(),
                &plan_id,
                sub.cancel_at_period_end,
            );
        }
        EventObject::CustomerSubscriptionDeleted(sub) => {
            info!(
                %event_id,
                subscription_id = %sub.id,
                "Subscription deleted"
            );

            if let Err(e) = webhook::on_subscription_deleted(&state.db, &sub, &raw_data).await {
                error!(%event_id, error = %e, "Failed to revoke access after subscription deletion");
                return Err(e);
            }

            analytics::track_webhook_subscription_deleted();
        }
        EventObject::InvoicePaid(invoice) => {
            info!(
                %event_id,
                invoice_id = ?invoice.id,
                "Invoice paid"
            );

            let has_subscription = invoice.subscription.is_some();

            if let Err(e) = webhook::on_invoice_paid(&state.db, &invoice).await {
                error!(%event_id, error = %e, "Failed to handle invoice paid event");
                return Err(e);
            }

            analytics::track_webhook_invoice_paid(has_subscription);
        }
        EventObject::InvoicePaymentFailed(invoice) => {
            info!(
                %event_id,
                invoice_id = ?invoice.id,
                "Invoice payment failed"
            );

            let attempt_count = invoice.attempt_count;

            if let Err(e) = webhook::on_invoice_payment_failed(&state.db, &invoice).await {
                error!(%event_id, error = %e, "Failed to handle invoice payment failure");
                return Err(e);
            }

            analytics::track_webhook_invoice_payment_failed(attempt_count);
        }
        _ => {
            warn!(%event_id, %event_type, "Unhandled webhook event");
        }
    }

    Ok(StatusCode::OK)
}
