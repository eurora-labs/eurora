use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{LlmConfig, Provider, ProviderId, Roles};

/// View of [`LlmConfig`] safe to serialise across the wire and into logs.
///
/// Mirrors the shape of [`LlmConfig`] but replaces every secret-bearing field
/// with a boolean flag indicating whether a value is present. Consumers
/// (e.g. the desktop app's connection panel) can show "you're connected to
/// OpenAI / gpt-4o-mini" without ever seeing the API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RedactedLlmConfig {
    pub providers: HashMap<ProviderId, RedactedProvider>,
    pub roles: Roles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind")]
pub enum RedactedProvider {
    #[serde(rename = "openai")]
    OpenAI {
        has_api_key: bool,
        base_url: Option<Url>,
        organization: Option<String>,
    },
    #[serde(rename = "anthropic")]
    Anthropic {
        has_api_key: bool,
        base_url: Option<Url>,
    },
    #[serde(rename = "google")]
    Google {
        credentials: RedactedGoogleCreds,
        project: Option<String>,
    },
    #[serde(rename = "bedrock")]
    Bedrock {
        region: String,
        credentials: RedactedAwsCreds,
    },
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible {
        base_url: Url,
        has_api_key: bool,
        header_names: Vec<String>,
        has_overrides: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind")]
pub enum RedactedGoogleCreds {
    #[serde(rename = "api_key")]
    ApiKey { has_key: bool },
    #[serde(rename = "service_account")]
    ServiceAccount { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind")]
pub enum RedactedAwsCreds {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "static")]
    Static { access_key_id: String },
}

impl From<&LlmConfig> for RedactedLlmConfig {
    fn from(value: &LlmConfig) -> Self {
        let providers = value
            .providers
            .iter()
            .map(|(id, p)| (id.clone(), RedactedProvider::from(p)))
            .collect();
        Self {
            providers,
            roles: value.roles.clone(),
        }
    }
}

impl From<&Provider> for RedactedProvider {
    fn from(value: &Provider) -> Self {
        use crate::{AwsCreds, GoogleCreds};
        match value {
            Provider::OpenAI {
                base_url,
                organization,
                ..
            } => RedactedProvider::OpenAI {
                has_api_key: true,
                base_url: base_url.clone(),
                organization: organization.clone(),
            },
            Provider::Anthropic { base_url, .. } => RedactedProvider::Anthropic {
                has_api_key: true,
                base_url: base_url.clone(),
            },
            Provider::Google {
                credentials,
                project,
            } => RedactedProvider::Google {
                credentials: match credentials {
                    GoogleCreds::ApiKey { .. } => RedactedGoogleCreds::ApiKey { has_key: true },
                    GoogleCreds::ServiceAccount { path } => {
                        RedactedGoogleCreds::ServiceAccount { path: path.clone() }
                    }
                },
                project: project.clone(),
            },
            Provider::Bedrock {
                region,
                credentials,
            } => RedactedProvider::Bedrock {
                region: region.clone(),
                credentials: match credentials {
                    AwsCreds::Default => RedactedAwsCreds::Default,
                    AwsCreds::Static { access_key_id, .. } => RedactedAwsCreds::Static {
                        access_key_id: access_key_id.clone(),
                    },
                },
            },
            Provider::OpenAiCompatible {
                base_url,
                api_key,
                headers,
                overrides,
            } => RedactedProvider::OpenAiCompatible {
                base_url: base_url.clone(),
                has_api_key: api_key.is_some(),
                header_names: headers.keys().cloned().collect(),
                has_overrides: !overrides.is_empty(),
            },
        }
    }
}
