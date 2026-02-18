use co_utils::Sensitive;
use url::Url;

use crate::error::{Error, Result};
use crate::proto;

#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub base_url: Url,
    pub api_key: Sensitive<String>,
    pub model: String,
    pub title_model: Option<String>,
}

impl TryFrom<proto::OpenAiSettings> for OpenAIConfig {
    type Error = Error;

    fn try_from(p: proto::OpenAiSettings) -> Result<Self> {
        if p.base_url.is_empty() {
            return Err(Error::EmptyField("base_url"));
        }
        if p.api_key.is_empty() {
            return Err(Error::EmptyField("api_key"));
        }
        if p.model.is_empty() {
            return Err(Error::EmptyField("model"));
        }
        let title_model = if p.title_model.is_empty() {
            None
        } else {
            Some(p.title_model)
        };
        Ok(Self {
            base_url: Url::parse(&p.base_url)?,
            api_key: Sensitive(p.api_key),
            model: p.model,
            title_model,
        })
    }
}
