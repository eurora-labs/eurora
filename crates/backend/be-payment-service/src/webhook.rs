use std::sync::Arc;

use be_remote_db::DatabaseManager;
use stripe_shared::Subscription;

use crate::error::PaymentError;

fn serialize_or_null(value: &impl serde::Serialize, context: &str) -> serde_json::Value {
    match serde_json::to_value(value) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, context, "Failed to serialize Stripe object, storing null");
            serde_json::Value::Null
        }
    }
}

fn extract_subscription_items(
    sub: &Subscription,
) -> Vec<(String, String, Option<i64>, serde_json::Value)> {
    sub.items
        .data
        .iter()
        .map(|item| {
            let raw = serialize_or_null(item, "subscription_item");
            (
                item.id.to_string(),
                item.price.id.to_string(),
                item.quantity.map(|q| q as i64),
                raw,
            )
        })
        .collect()
}

/// Extract the current billing period from the first subscription item.
/// Falls back to (0, 0) if there are no items.
fn extract_period(sub: &Subscription) -> (i64, i64) {
    sub.items
        .data
        .first()
        .map(|item| (item.current_period_start, item.current_period_end))
        .unwrap_or((0, 0))
}

fn extract_first_price_id(sub: &Subscription) -> Option<String> {
    sub.items.data.first().map(|item| item.price.id.to_string())
}

pub async fn on_checkout_completed(
    db: &Arc<DatabaseManager>,
    customer_id: Option<String>,
    subscription_id: Option<String>,
    customer_email: Option<String>,
    subscription: Option<&Subscription>,
    raw_data: &serde_json::Value,
) -> Result<(), PaymentError> {
    let customer_id =
        customer_id.ok_or_else(|| PaymentError::MissingField("customer_id in checkout session"))?;

    let customer_email = customer_email
        .ok_or_else(|| PaymentError::MissingField("customer_email in checkout session"))?;

    tracing::info!(
        %customer_id,
        ?subscription_id,
        %customer_email,
        "Checkout completed — provisioning"
    );

    // Look up the application user; if no user exists for this email,
    // we still upsert the Stripe customer (so the data is captured)
    // but skip account/subscription provisioning.
    let user = match db.get_user_by_email(&customer_email).await {
        Ok(u) => Some(u),
        Err(e) if e.is_not_found() => {
            tracing::warn!(
                %customer_email,
                "No application user found for checkout email — Stripe customer will be saved but account provisioning skipped"
            );
            None
        }
        Err(e) => return Err(anyhow::anyhow!("lookup user by email: {e}").into()),
    };

    let mut tx = db
        .pool
        .begin()
        .await
        .map_err(|e| anyhow::anyhow!("begin tx: {e}"))?;

    db.upsert_stripe_customer(&mut *tx, &customer_id, Some(&customer_email), raw_data)
        .await
        .map_err(|e| anyhow::anyhow!("upsert stripe customer: {e}"))?;

    if let Some(user) = &user {
        db.ensure_account_for_user(&mut *tx, user.id, &customer_id)
            .await
            .map_err(|e| anyhow::anyhow!("ensure account: {e}"))?;
    }

    if let Some(ref sub_id) = subscription_id {
        let (period_start, period_end) = subscription.map(extract_period).unwrap_or((0, 0));
        let canceled_at = subscription.and_then(|s| s.canceled_at);
        let cancel_at_period_end = subscription
            .map(|s| s.cancel_at_period_end)
            .unwrap_or(false);

        let sub_raw = subscription
            .map(|s| serialize_or_null(s, "checkout_subscription"))
            .unwrap_or(serde_json::Value::Null);

        db.upsert_stripe_subscription()
            .executor(&mut *tx)
            .subscription_id(sub_id)
            .customer_id(&customer_id)
            .status("active")
            .cancel_at_period_end(cancel_at_period_end)
            .maybe_canceled_at(canceled_at)
            .current_period_start(period_start)
            .current_period_end(period_end)
            .raw_data(&sub_raw)
            .call()
            .await
            .map_err(|e| anyhow::anyhow!("upsert subscription: {e}"))?;

        if let Some(sub) = subscription {
            let items = extract_subscription_items(sub);
            db.sync_stripe_subscription_items(&mut *tx, sub_id, &items)
                .await
                .map_err(|e| anyhow::anyhow!("sync subscription items: {e}"))?;

            if let Some(price_id) = extract_first_price_id(sub)
                && let Ok(Some(plan_id)) =
                    db.resolve_plan_for_stripe_price(&mut *tx, &price_id).await
            {
                db.update_account_plan_by_stripe_customer(&mut *tx, &customer_id, &plan_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("update account plan: {e}"))?;
            }
        }
    } else {
        tracing::warn!(
            %customer_id,
            %customer_email,
            "Checkout completed without a subscription_id — subscription provisioning skipped"
        );
    }

    tx.commit()
        .await
        .map_err(|e| anyhow::anyhow!("commit tx: {e}"))?;

    Ok(())
}

pub async fn on_subscription_updated(
    db: &Arc<DatabaseManager>,
    sub: &Subscription,
    _raw_data: &serde_json::Value,
) -> Result<(), PaymentError> {
    let subscription_id = sub.id.to_string();
    let customer_id = sub.customer.id().to_string();
    let status = sub.status.to_string();
    let cancel_at_period_end = sub.cancel_at_period_end;
    let canceled_at = sub.canceled_at;
    let (period_start, period_end) = extract_period(sub);

    tracing::info!(
        %subscription_id,
        %customer_id,
        %status,
        "Subscription updated — syncing status"
    );

    let sub_raw = serialize_or_null(sub, "subscription_updated");

    let mut tx = db
        .pool
        .begin()
        .await
        .map_err(|e| anyhow::anyhow!("begin tx: {e}"))?;

    db.upsert_stripe_subscription()
        .executor(&mut *tx)
        .subscription_id(&subscription_id)
        .customer_id(&customer_id)
        .status(&status)
        .cancel_at_period_end(cancel_at_period_end)
        .maybe_canceled_at(canceled_at)
        .current_period_start(period_start)
        .current_period_end(period_end)
        .raw_data(&sub_raw)
        .call()
        .await
        .map_err(|e| anyhow::anyhow!("upsert subscription: {e}"))?;

    let items = extract_subscription_items(sub);
    db.sync_stripe_subscription_items(&mut *tx, &subscription_id, &items)
        .await
        .map_err(|e| anyhow::anyhow!("sync subscription items: {e}"))?;

    let plan_id = if matches!(status.as_str(), "active" | "trialing") {
        let resolved = if let Some(price_id) = extract_first_price_id(sub) {
            db.resolve_plan_for_stripe_price(&mut *tx, &price_id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };
        resolved.unwrap_or_else(|| "free".to_string())
    } else {
        "free".to_string()
    };

    db.update_account_plan_by_stripe_customer(&mut *tx, &customer_id, &plan_id)
        .await
        .map_err(|e| anyhow::anyhow!("update account plan: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| anyhow::anyhow!("commit tx: {e}"))?;

    Ok(())
}

pub async fn resolve_plan_id_for_tracking(db: &Arc<DatabaseManager>, sub: &Subscription) -> String {
    let status = sub.status.to_string();
    if matches!(status.as_str(), "active" | "trialing")
        && let Some(price_id) = extract_first_price_id(sub)
        && let Ok(Some(plan_id)) = db.resolve_plan_for_stripe_price(&db.pool, &price_id).await
    {
        return plan_id;
    }
    "free".to_string()
}

pub async fn on_subscription_deleted(
    db: &Arc<DatabaseManager>,
    sub: &Subscription,
    _raw_data: &serde_json::Value,
) -> Result<(), PaymentError> {
    let subscription_id = sub.id.to_string();
    let customer_id = sub.customer.id().to_string();
    let canceled_at = sub.canceled_at;

    tracing::info!(
        %subscription_id,
        "Subscription deleted — revoking"
    );

    let sub_raw = serialize_or_null(sub, "subscription_deleted");

    let mut tx = db
        .pool
        .begin()
        .await
        .map_err(|e| anyhow::anyhow!("begin tx: {e}"))?;

    db.update_stripe_subscription_status_with_executor(
        &mut *tx,
        &subscription_id,
        "canceled",
        false,
        canceled_at,
        &sub_raw,
    )
    .await
    .map_err(|e| anyhow::anyhow!("update subscription status: {e}"))?;

    db.update_account_plan_by_stripe_customer(&mut *tx, &customer_id, "free")
        .await
        .map_err(|e| anyhow::anyhow!("reset account plan: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| anyhow::anyhow!("commit tx: {e}"))?;

    Ok(())
}

pub async fn on_invoice_paid(
    _db: &Arc<DatabaseManager>,
    invoice: &stripe_shared::Invoice,
) -> Result<(), PaymentError> {
    let invoice_id = invoice
        .id
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let subscription_id = invoice.subscription.as_ref().map(|s| s.id().to_string());

    tracing::info!(
        %invoice_id,
        subscription_id = ?subscription_id,
        "Invoice paid — subscription renewal confirmed"
    );

    Ok(())
}

pub async fn on_invoice_payment_failed(
    _db: &Arc<DatabaseManager>,
    invoice: &stripe_shared::Invoice,
) -> Result<(), PaymentError> {
    let invoice_id = invoice
        .id
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let subscription_id = invoice.subscription.as_ref().map(|s| s.id().to_string());
    let attempt_count = invoice.attempt_count;

    tracing::warn!(
        %invoice_id,
        subscription_id = ?subscription_id,
        attempt_count,
        "Invoice payment failed"
    );

    Ok(())
}
