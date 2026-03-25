use crate::error::{Error, Result};
use euro_secret::{SecretString, secret};
use serde::{Deserialize, Serialize};
use specta::Type;

const OPENAI_API_KEY_HANDLE: &str = "OPENAI_API_KEY";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
pub struct OpenAISettings {
    pub base_url: String,
    pub model: String,
    pub title_model: Option<String>,
}

impl OpenAISettings {
    fn api_key() -> Result<Option<SecretString>> {
        secret::retrieve(OPENAI_API_KEY_HANDLE).map_err(|e| Error::Secret(e.to_string()))
    }

    pub fn set_api_key(api_key: &str) -> Result<()> {
        secret::persist(
            OPENAI_API_KEY_HANDLE,
            &SecretString::from(api_key.to_owned()),
        )
        .map_err(|e| Error::Secret(e.to_string()))
    }
}
