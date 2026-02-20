use std::time::Duration;

use posthog_rs::Event;
use tracing::warn;

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);

fn capture_async(event: Event) {
    tokio::spawn(async move {
        match tokio::time::timeout(CAPTURE_TIMEOUT, posthog_rs::capture(event)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => warn!("Failed to capture analytics event: {e}"),
            Err(_) => warn!("Analytics event capture timed out"),
        }
    });
}

pub fn track_checkout_session_created(price_id: &str) {
    let mut event = Event::new_anon("checkout_session_created");
    event.insert_prop("price_id", price_id).ok();
    capture_async(event);
}

pub fn track_checkout_session_creation_failed(price_id: Option<&str>, error_kind: &str) {
    let mut event = Event::new_anon("checkout_session_creation_failed");
    if let Some(pid) = price_id {
        event.insert_prop("price_id", pid).ok();
    }
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}

pub fn track_checkout_status_checked(status: &str) {
    let mut event = Event::new_anon("checkout_status_checked");
    event.insert_prop("status", status).ok();
    capture_async(event);
}

pub fn track_billing_portal_created() {
    let event = Event::new_anon("billing_portal_created");
    capture_async(event);
}

pub fn track_billing_portal_failed(error_kind: &str) {
    let mut event = Event::new_anon("billing_portal_failed");
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}

pub fn track_subscription_status_checked(status: Option<&str>, price_id: Option<&str>) {
    let mut event = Event::new_anon("subscription_status_checked");
    event.insert_prop("status", status.unwrap_or("none")).ok();
    if let Some(pid) = price_id {
        event.insert_prop("price_id", pid).ok();
    }
    capture_async(event);
}

pub fn track_webhook_checkout_completed(has_subscription: bool, has_user: bool) {
    let mut event = Event::new_anon("webhook_checkout_completed");
    event.insert_prop("has_subscription", has_subscription).ok();
    event.insert_prop("has_user", has_user).ok();
    capture_async(event);
}

pub fn track_webhook_subscription_updated(status: &str, plan_id: &str, cancel_at_period_end: bool) {
    let mut event = Event::new_anon("webhook_subscription_updated");
    event.insert_prop("status", status).ok();
    event.insert_prop("plan_id", plan_id).ok();
    event
        .insert_prop("cancel_at_period_end", cancel_at_period_end)
        .ok();
    capture_async(event);
}

pub fn track_webhook_subscription_deleted() {
    let event = Event::new_anon("webhook_subscription_deleted");
    capture_async(event);
}

pub fn track_webhook_invoice_paid(has_subscription: bool) {
    let mut event = Event::new_anon("webhook_invoice_paid");
    event.insert_prop("has_subscription", has_subscription).ok();
    capture_async(event);
}

pub fn track_webhook_invoice_payment_failed(attempt_count: u64) {
    let mut event = Event::new_anon("webhook_invoice_payment_failed");
    event.insert_prop("attempt_count", attempt_count).ok();
    capture_async(event);
}
