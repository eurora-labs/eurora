use euro_secret::ExposeSecret;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::error::ResultExt;
use crate::shared_types::SharedUserController;

fn rest_api_url() -> String {
    std::env::var("REST_API_URL")
        .or_else(|_| std::env::var("API_BASE_URL"))
        .unwrap_or_else(|_| "https://api.eurora-labs.com".to_string())
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

#[taurpc::procedures(path = "payment")]
pub trait PaymentApi {
    async fn create_checkout_url<R: Runtime>(app_handle: AppHandle<R>) -> Result<String, String>;
}

#[derive(Clone)]
pub struct PaymentApiImpl;

#[taurpc::resolvers]
impl PaymentApi for PaymentApiImpl {
    async fn create_checkout_url<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
    ) -> Result<String, String> {
        let user_state = app_handle
            .try_state::<SharedUserController>()
            .ok_or_else(|| "User controller not available".to_string())?;

        let token = {
            let mut controller = user_state.lock().await;
            controller
                .get_or_refresh_access_token()
                .await
                .ctx("Failed to get access token")?
        };

        let base_url = rest_api_url();
        let client = Client::new();

        let pricing: PricingResponse = client
            .get(format!("{base_url}/payment/pricing"))
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
            .post(format!("{base_url}/payment/checkout"))
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
}
