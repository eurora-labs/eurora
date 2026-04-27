use std::collections::HashMap;
use std::sync::Arc;

use be_remote_db::DatabaseManager;
use stripe::{IdempotencyKey, RequestStrategy, StripeError, StripeRequest};
use stripe_core::customer::{CreateCustomer, ListCustomer};
use stripe_shared::Customer;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ProvisionError {
    #[error("transient provisioning error: {0}")]
    Transient(String),

    #[error("permanent provisioning error: {0}")]
    Permanent(String),
}

impl ProvisionError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Transient(_) => "transient",
            Self::Permanent(_) => "permanent",
        }
    }
}

/// Stripe-backed customer provisioner that syncs Stripe state into the
/// application database.
///
/// `ensure_customer` is idempotent on `user_id`: repeated calls converge on a
/// single Stripe customer and a single `users.stripe_customer_id` link. The
/// drainer relies on this for retry safety.
///
/// Holds its own `stripe::Client` and `Arc<DatabaseManager>` rather than a
/// reference to `AppState` so the application state can in turn carry an
/// `Arc<StripeBillingProvisioner>` without forming a reference cycle.
pub struct StripeBillingProvisioner {
    client: stripe::Client,
    db: Arc<DatabaseManager>,
}

impl StripeBillingProvisioner {
    pub fn new(client: stripe::Client, db: Arc<DatabaseManager>) -> Self {
        Self { client, db }
    }

    pub async fn ensure_customer(
        &self,
        user_id: Uuid,
        email: &str,
    ) -> Result<String, ProvisionError> {
        if let Some(existing) = self
            .db
            .get_stripe_customer_id_for_user()
            .user_id(user_id)
            .call()
            .await
            .map_err(|e| ProvisionError::Transient(format!("lookup link: {e}")))?
        {
            return Ok(existing);
        }

        // Defence against duplicates: a customer may already exist in Stripe
        // under this email (e.g. created out-of-band or by a previous code
        // path) without a local link. Reuse it instead of creating a duplicate.
        let existing_in_stripe = ListCustomer::new()
            .email(email)
            .limit(1)
            .send(&self.client)
            .await
            .map_err(stripe_to_provision_error)?;

        let customer: Customer = if let Some(existing) = existing_in_stripe.data.into_iter().next()
        {
            existing
        } else {
            // Idempotency-Key = the user's UUID. Retries from the same caller
            // (or the drainer after a transient failure) collapse to a single
            // customer even if Stripe returned the response and we never
            // observed it.
            let key = IdempotencyKey::from(user_id);
            let metadata = HashMap::from([("app_user_id".to_string(), user_id.to_string())]);
            CreateCustomer::new()
                .email(email)
                .metadata(metadata)
                .customize()
                .request_strategy(RequestStrategy::Idempotent(key))
                .send(&self.client)
                .await
                .map_err(stripe_to_provision_error)?
        };

        let customer_id = customer.id.to_string();
        let raw_data = serde_json::to_value(&customer).unwrap_or(serde_json::Value::Null);

        persist_link(&self.db, user_id, &customer_id, email, &raw_data)
            .await
            .map_err(|e| ProvisionError::Transient(format!("persist link: {e}")))?;

        tracing::info!(%user_id, %customer_id, "Provisioned Stripe customer");

        Ok(customer_id)
    }
}

async fn persist_link(
    db: &DatabaseManager,
    user_id: Uuid,
    customer_id: &str,
    email: &str,
    raw_data: &serde_json::Value,
) -> anyhow::Result<()> {
    let mut tx = db.pool.begin().await?;

    db.upsert_stripe_customer()
        .executor(&mut *tx)
        .customer_id(customer_id)
        .app_user_id(user_id)
        .email(email)
        .raw_data(raw_data)
        .call()
        .await?;

    db.link_stripe_customer_to_user()
        .executor(&mut *tx)
        .user_id(user_id)
        .stripe_customer_id(customer_id)
        .call()
        .await?;

    tx.commit().await?;
    Ok(())
}

fn stripe_to_provision_error(err: StripeError) -> ProvisionError {
    match &err {
        StripeError::Stripe(_, status) if (400..500).contains(status) && *status != 429 => {
            ProvisionError::Permanent(err.to_string())
        }
        _ => ProvisionError::Transient(err.to_string()),
    }
}
