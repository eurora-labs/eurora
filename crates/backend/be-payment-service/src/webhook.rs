/// Events dispatched by the webhook handler when Stripe notifies us of payment lifecycle changes.
///
/// Implement this trait to wire up provisioning (grant/revoke access, update DB records, etc).
#[allow(unused_variables)]
pub trait WebhookEventHandler: Send + Sync + 'static {
    /// A checkout session was completed — the customer has paid.
    ///
    /// Use `customer_id` and `subscription_id` to link the Stripe subscription to
    /// your internal user record and provision access.
    fn on_checkout_completed(
        &self,
        customer_id: Option<String>,
        subscription_id: Option<String>,
        customer_email: Option<String>,
    ) -> impl std::future::Future<Output = Result<(), crate::error::PaymentError>> + Send {
        async { Ok(()) }
    }

    /// A subscription was updated (e.g. plan change, renewal, payment failure recovery).
    fn on_subscription_updated(
        &self,
        subscription_id: String,
        customer_id: Option<String>,
        status: String,
    ) -> impl std::future::Future<Output = Result<(), crate::error::PaymentError>> + Send {
        async { Ok(()) }
    }

    /// A subscription was deleted/cancelled — revoke the customer's access.
    fn on_subscription_deleted(
        &self,
        subscription_id: String,
        customer_id: Option<String>,
    ) -> impl std::future::Future<Output = Result<(), crate::error::PaymentError>> + Send {
        async { Ok(()) }
    }
}

/// Default handler that logs webhook events without taking any action.
///
/// Used when no custom handler is provided.
pub struct LoggingWebhookHandler;

impl WebhookEventHandler for LoggingWebhookHandler {
    async fn on_checkout_completed(
        &self,
        customer_id: Option<String>,
        subscription_id: Option<String>,
        customer_email: Option<String>,
    ) -> Result<(), crate::error::PaymentError> {
        tracing::info!(
            ?customer_id,
            ?subscription_id,
            ?customer_email,
            "Checkout completed (no-op handler)"
        );
        Ok(())
    }

    async fn on_subscription_updated(
        &self,
        subscription_id: String,
        customer_id: Option<String>,
        status: String,
    ) -> Result<(), crate::error::PaymentError> {
        tracing::info!(
            %subscription_id,
            ?customer_id,
            %status,
            "Subscription updated (no-op handler)"
        );
        Ok(())
    }

    async fn on_subscription_deleted(
        &self,
        subscription_id: String,
        customer_id: Option<String>,
    ) -> Result<(), crate::error::PaymentError> {
        tracing::info!(
            %subscription_id,
            ?customer_id,
            "Subscription deleted (no-op handler)"
        );
        Ok(())
    }
}
