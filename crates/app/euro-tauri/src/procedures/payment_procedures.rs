use euro_secret::ExposeSecret;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use url::Url;

use crate::error::ResultExt;
use crate::procedures::auth_manager;
use crate::shared_types::SharedEndpointManager;

/// Build an absolute URL for `path` against the shared
/// [`EndpointManager`]. The manager is the single source of truth for
/// which backend the rest of the desktop is talking to (auth,
/// threads, timeline) — payment must hit the same one or the
/// Default/Custom toggle in `Settings → API` silently splits the app
/// across two backends.
fn api_url(app_handle: &AppHandle, path: &str) -> Url {
    app_handle.state::<SharedEndpointManager>().url(path)
}

#[derive(Deserialize)]
struct PricingResponse {
    pro_price_id: String,
}

#[derive(Serialize)]
struct CheckoutRequest {
    price_id: String,
}

#[derive(Deserialize)]
struct CheckoutResponse {
    url: String,
}

#[derive(Deserialize)]
struct SubscriptionResponse {
    subscription_id: Option<String>,
    status: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn payment_create_checkout_url(app_handle: AppHandle) -> Result<String, String> {
    let auth_manager = auth_manager(&app_handle).await?;
    let token = auth_manager
        .get_or_refresh_access_token()
        .await
        .ctx("Failed to get access token")?;

    let client = Client::new();

    let pricing: PricingResponse = client
        .get(api_url(&app_handle, "/payment/pricing"))
        .header("Authorization", format!("Bearer {}", token.expose_secret()))
        .send()
        .await
        .ctx("Failed to fetch pricing")?
        .error_for_status()
        .ctx("Pricing request failed")?
        .json()
        .await
        .ctx("Failed to parse pricing response")?;

    let checkout: CheckoutResponse = client
        .post(api_url(&app_handle, "/payment/checkout"))
        .header("Authorization", format!("Bearer {}", token.expose_secret()))
        .json(&CheckoutRequest {
            price_id: pricing.pro_price_id,
        })
        .send()
        .await
        .ctx("Failed to create checkout session")?
        .error_for_status()
        .ctx("Checkout request failed")?
        .json()
        .await
        .ctx("Failed to parse checkout response")?;

    Ok(checkout.url)
}

#[tauri::command]
#[specta::specta]
pub async fn payment_is_subscribed(app_handle: AppHandle) -> Result<bool, String> {
    let auth_manager = auth_manager(&app_handle).await?;
    let token = auth_manager
        .get_or_refresh_access_token()
        .await
        .ctx("Failed to get access token")?;

    let sub: SubscriptionResponse = Client::new()
        .get(api_url(&app_handle, "/payment/subscription"))
        .header("Authorization", format!("Bearer {}", token.expose_secret()))
        .send()
        .await
        .ctx("Failed to fetch subscription status")?
        .error_for_status()
        .ctx("Subscription request failed")?
        .json()
        .await
        .ctx("Failed to parse subscription response")?;

    Ok(sub.subscription_id.is_some() && sub.status.as_deref() == Some("active"))
}
