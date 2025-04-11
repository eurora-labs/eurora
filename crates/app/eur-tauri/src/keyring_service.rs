use anyhow::Result;
use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "eurora";
const API_KEY_USERNAME: &str = "openai_api_key";

/// Manages secure storage of API keys using the system keyring
pub struct KeyringService;

impl KeyringService {
    /// Creates a new KeyringService
    pub fn new() -> Self {
        Self
    }

    /// Checks if an OpenAI API key is stored in the keyring
    pub fn has_api_key(&self) -> bool {
        self.get_api_key().is_ok()
    }

    /// Retrieves the OpenAI API key from the keyring
    pub fn get_api_key(&self) -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, API_KEY_USERNAME)?;
        let key = entry.get_password()?;
        Ok(key)
    }

    /// Stores the OpenAI API key in the keyring
    pub fn set_api_key(&self, api_key: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, API_KEY_USERNAME)?;
        entry.set_password(api_key)?;
        Ok(())
    }

    /// Deletes the OpenAI API key from the keyring
    pub fn delete_api_key(&self) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, API_KEY_USERNAME)?;
        entry.delete_password()?;
        Ok(())
    }
}

/// Response for API key status
#[derive(Serialize, Deserialize)]
pub struct ApiKeyStatus {
    pub has_key: bool,
}