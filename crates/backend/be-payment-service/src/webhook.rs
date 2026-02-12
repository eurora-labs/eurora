use std::sync::Arc;

use be_remote_db::DatabaseManager;
use tracing::info;

use crate::error::PaymentError;

pub async fn on_checkout_completed(
    db: &Arc<DatabaseManager>,
    customer_id: Option<String>,
    subscription_id: Option<String>,
    customer_email: Option<String>,
) -> Result<(), PaymentError> {
    let customer_id =
        customer_id.ok_or_else(|| PaymentError::MissingField("customer_id in checkout session"))?;

    info!(
        %customer_id,
        ?subscription_id,
        ?customer_email,
        "Checkout completed — provisioning"
    );

    db.upsert_stripe_customer(&customer_id, customer_email.as_deref())
        .await
        .map_err(|e| anyhow::anyhow!("upsert stripe customer: {e}"))?;

    if let Some(ref email) = customer_email
        && let Ok(user) = db.get_user_by_email(email).await
    {
        db.ensure_account_for_user(user.id, &customer_id)
            .await
            .map_err(|e| anyhow::anyhow!("ensure account: {e}"))?;
    }

    if let Some(ref sub_id) = subscription_id {
        db.upsert_stripe_subscription(sub_id, &customer_id, "active")
            .await
            .map_err(|e| anyhow::anyhow!("upsert subscription: {e}"))?;
    }

    Ok(())
}

pub async fn on_subscription_updated(
    db: &Arc<DatabaseManager>,
    subscription_id: String,
    customer_id: Option<String>,
    status: String,
) -> Result<(), PaymentError> {
    info!(
        %subscription_id,
        ?customer_id,
        %status,
        "Subscription updated — syncing status"
    );

    if let Some(ref cust_id) = customer_id {
        db.upsert_stripe_subscription(&subscription_id, cust_id, &status)
            .await
            .map_err(|e| anyhow::anyhow!("upsert subscription: {e}"))?;
    } else {
        db.update_stripe_subscription_status(&subscription_id, &status)
            .await
            .map_err(|e| anyhow::anyhow!("update subscription status: {e}"))?;
    }

    Ok(())
}

pub async fn on_subscription_deleted(
    db: &Arc<DatabaseManager>,
    subscription_id: String,
    customer_id: Option<String>,
) -> Result<(), PaymentError> {
    info!(
        %subscription_id,
        ?customer_id,
        "Subscription deleted — revoking"
    );

    db.update_stripe_subscription_status(&subscription_id, "canceled")
        .await
        .map_err(|e| anyhow::anyhow!("update subscription status: {e}"))?;

    Ok(())
}
