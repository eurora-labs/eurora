use crate::{
    error::{Result, StripeServiceError},
    service::StripeService,
};
use axum::{body::Bytes, extract::State, http::HeaderMap, response::Json};
use serde_json::{Value, json};
use stripe::{Event, EventObject, EventType, Webhook};

pub async fn handle_webhook(
    State(service): State<StripeService>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>> {
    let payload = String::from_utf8(body.to_vec())
        .map_err(|_| StripeServiceError::WebhookVerificationFailed("Invalid UTF-8".to_string()))?;

    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            StripeServiceError::WebhookVerificationFailed("Missing signature".to_string())
        })?;

    // Verify webhook signature
    let event =
        Webhook::construct_event(&payload, signature, &service.config.stripe.webhook_secret)
            .map_err(|e| StripeServiceError::WebhookVerificationFailed(e.to_string()))?;

    // Store webhook event in database
    let webhook_request = eur_remote_db::CreateWebhookEventRequest {
        stripe_event_id: event.id.to_string(),
        event_type: event.type_.to_string(),
        api_version: event.api_version.clone(),
        data: serde_json::to_value(&event.data).unwrap_or_default(),
    };

    if let Err(e) = service.db.create_webhook_event(webhook_request).await {
        tracing::error!("Failed to store webhook event: {:?}", e);
    }

    // Process the event
    match process_webhook_event(&service, &event).await {
        Ok(_) => {
            tracing::info!("Successfully processed webhook event: {}", event.id);
        }
        Err(e) => {
            tracing::error!("Failed to process webhook event {}: {:?}", event.id, e);
            // Don't return error to Stripe, as we've already stored the event
        }
    }

    Ok(Json(json!({ "received": true })))
}

async fn process_webhook_event(service: &StripeService, event: &Event) -> Result<()> {
    match event.type_ {
        EventType::CustomerCreated => {
            if let EventObject::Customer(customer) = &event.data.object {
                tracing::info!("Customer created: {}", customer.id);
                // Additional processing if needed
            }
        }
        EventType::CustomerUpdated => {
            if let EventObject::Customer(customer) = &event.data.object {
                tracing::info!("Customer updated: {}", customer.id);
                // Update customer in database if needed
            }
        }
        EventType::CustomerDeleted => {
            if let EventObject::Customer(customer) = &event.data.object {
                tracing::info!("Customer deleted: {}", customer.id);
                // Handle customer deletion
            }
        }
        EventType::InvoiceCreated => {
            if let EventObject::Invoice(invoice) = &event.data.object {
                tracing::info!("Invoice created: {}", invoice.id);
                // Store invoice in database
            }
        }
        EventType::InvoiceUpdated => {
            if let EventObject::Invoice(invoice) = &event.data.object {
                tracing::info!("Invoice updated: {}", invoice.id);
                // Update invoice in database
            }
        }
        EventType::InvoicePaymentSucceeded => {
            if let EventObject::Invoice(invoice) = &event.data.object {
                tracing::info!("Invoice payment succeeded: {}", invoice.id);
                // Handle successful payment
            }
        }
        EventType::InvoicePaymentFailed => {
            if let EventObject::Invoice(invoice) = &event.data.object {
                tracing::info!("Invoice payment failed: {}", invoice.id);
                // Handle failed payment
            }
        }
        EventType::CustomerSubscriptionCreated => {
            if let EventObject::Subscription(subscription) = &event.data.object {
                tracing::info!("Subscription created: {}", subscription.id);
                // Store subscription in database
            }
        }
        EventType::CustomerSubscriptionUpdated => {
            if let EventObject::Subscription(subscription) = &event.data.object {
                tracing::info!("Subscription updated: {}", subscription.id);
                // Update subscription in database
            }
        }
        EventType::CustomerSubscriptionDeleted => {
            if let EventObject::Subscription(subscription) = &event.data.object {
                tracing::info!("Subscription deleted: {}", subscription.id);
                // Handle subscription cancellation
            }
        }
        EventType::PaymentIntentSucceeded => {
            if let EventObject::PaymentIntent(payment_intent) = &event.data.object {
                tracing::info!("Payment intent succeeded: {}", payment_intent.id);
                // Handle successful payment
            }
        }
        EventType::PaymentIntentPaymentFailed => {
            if let EventObject::PaymentIntent(payment_intent) = &event.data.object {
                tracing::info!("Payment intent failed: {}", payment_intent.id);
                // Handle failed payment
            }
        }
        EventType::CheckoutSessionCompleted => {
            if let EventObject::CheckoutSession(session) = &event.data.object {
                tracing::info!("Checkout session completed: {}", session.id);
                // Handle completed checkout
            }
        }
        _ => {
            tracing::info!("Unhandled webhook event type: {:?}", event.type_);
        }
    }

    Ok(())
}
