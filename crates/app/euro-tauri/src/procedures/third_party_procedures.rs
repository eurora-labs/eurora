use euro_secret::{Sensitive, secret};
#[taurpc::procedures(path = "third_party")]
pub trait ThirdPartyApi {
    async fn check_api_key_exists() -> Result<bool, String>;
    async fn save_api_key(api_key: String) -> Result<(), String>;
}

#[derive(Clone)]
pub struct ThirdPartyApiImpl;

#[taurpc::resolvers]
impl ThirdPartyApi for ThirdPartyApiImpl {
    async fn check_api_key_exists(self) -> Result<bool, String> {
        let key = secret::retrieve("OPENAI_API_KEY")
            .map_err(|e| format!("Failed to retrieve API key: {}", e))?;

        let key = key.map(|s| s.into_inner());

        if key.is_none() {
            return Ok(false);
        }

        Ok(true)
    }

    async fn save_api_key(self, api_key: String) -> Result<(), String> {
        secret::persist("OPENAI_API_KEY", &Sensitive(api_key))
            .map_err(|e| format!("Failed to save API key: {}", e))?;
        Ok(())
    }
}
