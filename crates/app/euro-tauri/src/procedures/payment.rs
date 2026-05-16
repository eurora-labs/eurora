use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use thiserror::Error;
use url::Url;

use euro_auth::tauri::auth_manager;

use crate::procedures::auth::AuthError;
use crate::shared_types::{SharedEndpointManager, SharedHttpClient};

/// Typed error surface for the `payment_*` IPC commands. Externally
/// tagged so the JS side can branch on `error.type`. `Auth` re-uses the
/// auth surface so the same "session expired" handler in the frontend
/// catches both auth-touching commands and payment-touching commands
/// without a parallel mapping.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum PaymentError {
    #[error("auth: {0}")]
    Auth(AuthError),
    #[error("backend unreachable: {0}")]
    Backend(String),
    #[error("bad response: {0}")]
    BadResponse(String),
}

impl From<AuthError> for PaymentError {
    fn from(err: AuthError) -> Self {
        PaymentError::Auth(err)
    }
}

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

fn http_client(app_handle: &AppHandle) -> reqwest::Client {
    app_handle.state::<SharedHttpClient>().inner().clone()
}

async fn resolve_token(app_handle: &AppHandle) -> Result<SecretString, PaymentError> {
    let manager = auth_manager(app_handle).ok_or(AuthError::StateUnavailable("auth manager"))?;
    manager.get_or_refresh_access_token().await.map_err(|e| {
        if e.is_logged_out() {
            AuthError::NotAuthenticated.into()
        } else if e.is_transient() {
            AuthError::Backend(e.to_string()).into()
        } else {
            AuthError::Internal(e.to_string()).into()
        }
    })
}

#[tauri::command]
#[specta::specta]
pub async fn payment_create_checkout_url(app_handle: AppHandle) -> Result<String, PaymentError> {
    let token = resolve_token(&app_handle).await?;
    let client = http_client(&app_handle);

    let pricing: PricingResponse = client
        .get(api_url(&app_handle, "/payment/pricing"))
        .header("Authorization", format!("Bearer {}", token.expose_secret()))
        .send()
        .await
        .map_err(|e| PaymentError::Backend(format!("Failed to fetch pricing: {e}")))?
        .error_for_status()
        .map_err(|e| PaymentError::BadResponse(format!("Pricing request failed: {e}")))?
        .json()
        .await
        .map_err(|e| PaymentError::BadResponse(format!("Failed to parse pricing response: {e}")))?;

    let checkout: CheckoutResponse = client
        .post(api_url(&app_handle, "/payment/checkout"))
        .header("Authorization", format!("Bearer {}", token.expose_secret()))
        .json(&CheckoutRequest {
            price_id: pricing.pro_price_id,
        })
        .send()
        .await
        .map_err(|e| PaymentError::Backend(format!("Failed to create checkout session: {e}")))?
        .error_for_status()
        .map_err(|e| PaymentError::BadResponse(format!("Checkout request failed: {e}")))?
        .json()
        .await
        .map_err(|e| {
            PaymentError::BadResponse(format!("Failed to parse checkout response: {e}"))
        })?;

    Ok(checkout.url)
}

#[tauri::command]
#[specta::specta]
pub async fn payment_is_subscribed(app_handle: AppHandle) -> Result<bool, PaymentError> {
    let token = resolve_token(&app_handle).await?;

    let sub: SubscriptionResponse = http_client(&app_handle)
        .get(api_url(&app_handle, "/payment/subscription"))
        .header("Authorization", format!("Bearer {}", token.expose_secret()))
        .send()
        .await
        .map_err(|e| PaymentError::Backend(format!("Failed to fetch subscription status: {e}")))?
        .error_for_status()
        .map_err(|e| PaymentError::BadResponse(format!("Subscription request failed: {e}")))?
        .json()
        .await
        .map_err(|e| {
            PaymentError::BadResponse(format!("Failed to parse subscription response: {e}"))
        })?;

    Ok(sub.subscription_id.is_some() && sub.status.as_deref() == Some("active"))
}
