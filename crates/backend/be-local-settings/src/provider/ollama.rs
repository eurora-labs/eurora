use url::Url;

use crate::error::{Error, Result};
use crate::proto;

#[derive(Debug, Clone, PartialEq)]
pub struct OllamaConfig {
    pub base_url: Url,
    pub model: String,
}

impl TryFrom<proto::OllamaSettings> for OllamaConfig {
    type Error = Error;

    fn try_from(p: proto::OllamaSettings) -> Result<Self> {
        if p.model.is_empty() {
            return Err(Error::EmptyField("model"));
        }
        Ok(Self {
            base_url: Url::parse(&p.base_url)?,
            model: p.model,
        })
    }
}
