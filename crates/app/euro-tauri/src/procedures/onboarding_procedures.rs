//! Authentication procedures for the Tauri application.

#[taurpc::procedures(path = "onboarding")]
pub trait OnboardingApi {
    async fn get_browser_extension_download_url() -> Result<String, String>;
}

#[derive(Clone)]
pub struct OnboardingApiImpl;

#[taurpc::resolvers]
impl OnboardingApi for OnboardingApiImpl {
    async fn get_browser_extension_download_url(self) -> Result<String, String> {
        let base_url =
            std::env::var("AUTH_SERVICE_URL").unwrap_or("https://www.eurora-labs.com".to_string());

        Ok(format!("{}/download/browser-extension", base_url))
    }
}
