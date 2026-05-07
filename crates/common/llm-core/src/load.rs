use std::collections::HashMap;
use std::env;

use secrecy::SecretString;

use crate::{
    ConfigError, LlmConfig, ModelRef, Provider, ProviderId, ProviderKind, Roles, validate::validate,
};

/// Where the resolved configuration came from. Reported alongside the config
/// for startup logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource {
    Env,
}

const ENV_KIND: &str = "EURORA_LLM_KIND";
const ENV_OPENAI_KEY: &str = "OPENAI_API_KEY";
const ENV_OPENAI_ORG: &str = "EURORA_OPENAI_ORG";
const ENV_LLM_BASE_URL: &str = "EURORA_LLM_BASE_URL";
const ENV_LLM_API_KEY: &str = "EURORA_LLM_API_KEY";
const ENV_CHAT_MODEL: &str = "EURORA_CHAT_MODEL";
const ENV_TITLE_MODEL: &str = "EURORA_TITLE_MODEL";
const ENV_VISION_MODEL: &str = "EURORA_VISION_MODEL";

/// Load configuration from environment variables.
///
/// # Variable surface
///
/// Common to every kind:
///
/// - `EURORA_LLM_KIND` — `openai` (default) or `openai_compatible`. Other
///   kinds defined in [`crate::Provider`] (`anthropic`, `google`, `bedrock`)
///   are recognised by the schema but rejected here until their runtime
///   wiring lands.
/// - `EURORA_CHAT_MODEL` — required model name for the chat role.
/// - `EURORA_TITLE_MODEL` — title model. Defaults to `EURORA_CHAT_MODEL`.
/// - `EURORA_VISION_MODEL` — optional. When set, vision is enabled and
///   bound to the same provider as chat.
///
/// `openai`:
///
/// - `OPENAI_API_KEY` — required.
/// - `EURORA_LLM_BASE_URL` — optional override for the API base.
/// - `EURORA_OPENAI_ORG` — optional `OpenAI-Organization` value.
///
/// `openai_compatible`:
///
/// - `EURORA_LLM_BASE_URL` — required.
/// - `EURORA_LLM_API_KEY` — optional; many local servers don't need one.
///
/// # Single-provider shape
///
/// The env loader produces a config with exactly one provider, named after
/// its kind (`openai` or `openai_compatible`), and assigns every role to that
/// provider. Multi-provider configurations require a future config-file
/// loader; the schema in this crate already supports them.
pub fn from_env() -> Result<(LlmConfig, ConfigSource), ConfigError> {
    let kind = parse_kind()?;
    let chat_model = require_env(ENV_CHAT_MODEL)?;
    let title_model = optional_env(ENV_TITLE_MODEL).unwrap_or_else(|| chat_model.clone());
    let vision_model = optional_env(ENV_VISION_MODEL);

    let provider = build_provider(kind)?;
    // `ProviderKind::as_str` returns validator-clean ids by construction.
    let provider_id: ProviderId = kind.as_str().to_string();

    let mut providers = HashMap::with_capacity(1);
    providers.insert(provider_id.clone(), provider);

    let roles = Roles {
        chat: ModelRef {
            provider: provider_id.clone(),
            model: chat_model,
        },
        title: ModelRef {
            provider: provider_id.clone(),
            model: title_model,
        },
        vision: vision_model.map(|model| ModelRef {
            provider: provider_id,
            model,
        }),
    };

    let config = LlmConfig { providers, roles };
    validate(&config)?;
    Ok((config, ConfigSource::Env))
}

fn parse_kind() -> Result<ProviderKind, ConfigError> {
    let raw = optional_env(ENV_KIND).unwrap_or_else(|| "openai".to_string());
    match raw.as_str() {
        "openai" => Ok(ProviderKind::OpenAI),
        "openai_compatible" => Ok(ProviderKind::OpenAiCompatible),
        "anthropic" => Err(ConfigError::KindNotYetWired { kind: "anthropic" }),
        "google" => Err(ConfigError::KindNotYetWired { kind: "google" }),
        "bedrock" => Err(ConfigError::KindNotYetWired { kind: "bedrock" }),
        _ => Err(ConfigError::UnknownEnumValue {
            name: ENV_KIND,
            value: raw,
            expected: "openai | openai_compatible",
        }),
    }
}

fn build_provider(kind: ProviderKind) -> Result<Provider, ConfigError> {
    match kind {
        ProviderKind::OpenAI => {
            let api_key = SecretString::from(require_env(ENV_OPENAI_KEY)?);
            let base_url = parse_optional_url(ENV_LLM_BASE_URL)?;
            let organization = optional_env(ENV_OPENAI_ORG);
            Ok(Provider::OpenAI {
                api_key,
                base_url,
                organization,
            })
        }
        ProviderKind::OpenAiCompatible => {
            let base_url = parse_optional_url(ENV_LLM_BASE_URL)?
                .ok_or(ConfigError::OpenAiCompatibleBaseUrlRequired)?;
            let api_key = optional_env(ENV_LLM_API_KEY).map(SecretString::from);
            Ok(Provider::OpenAiCompatible {
                base_url,
                api_key,
                headers: HashMap::new(),
                overrides: Default::default(),
            })
        }
        ProviderKind::Anthropic | ProviderKind::Google | ProviderKind::Bedrock => {
            // parse_kind() already rejects these — keep the match exhaustive.
            Err(ConfigError::KindNotYetWired {
                kind: kind.as_str(),
            })
        }
    }
}

fn require_env(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(ConfigError::MissingEnv(name)),
    }
}

fn optional_env(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

fn parse_optional_url(name: &'static str) -> Result<Option<url::Url>, ConfigError> {
    let Some(raw) = optional_env(name) else {
        return Ok(None);
    };
    url::Url::parse(&raw)
        .map(Some)
        .map_err(|source| ConfigError::InvalidUrl { name, source })
}
