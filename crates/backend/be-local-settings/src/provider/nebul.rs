use co_utils::Sensitive;
use url::Url;

use crate::error::{Error, Result};
use crate::proto;

const BASE_NEBUL_URL: &str = "https://api.inference.nebul.io/v1";

#[derive(Debug, Clone)]
pub struct NebulConfig {
    base_url: Url,
    pub model: String,
    pub api_key: Sensitive<String>,
}

impl NebulConfig {
    pub fn new(model: String, api_key: Sensitive<String>) -> Self {
        Self {
            base_url: Url::parse(BASE_NEBUL_URL).unwrap(),
            model,
            api_key,
        }
    }
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
}

impl TryFrom<proto::NebulSettings> for NebulConfig {
    type Error = Error;

    fn try_from(p: proto::NebulSettings) -> Result<Self> {
        if p.model.is_empty() {
            return Err(Error::EmptyField("model"));
        }
        Ok(Self {
            base_url: Url::parse(BASE_NEBUL_URL).unwrap(),
            model: p.model,
            api_key: Sensitive(p.api_key),
        })
    }
}
