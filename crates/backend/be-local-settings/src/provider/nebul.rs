use co_utils::Sensitive;
use url::Url;

use crate::error::{Error, Result};
use crate::proto;

#[derive(Debug, Clone)]
pub struct NebulConfig {
    base_url: Url,
    pub model: String,
    pub api_key: Sensitive<String>,
    pub title_model: String,
}

impl NebulConfig {
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
            base_url: Url::parse("https://api.inference.nebul.io/v1")?,
            model: p.model,
            api_key: Sensitive(p.api_key),
            title_model: p.title_model,
        })
    }
}
