use crate::{LlmConfig, ProviderId};

/// Errors produced during config loading or validation.
///
/// Variants carry enough context to point a human at the offending field
/// without exposing secrets.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("environment variable `{0}` is required but not set")]
    MissingEnv(&'static str),

    #[error(
        "environment variable `{name}` has an unrecognised value `{value}` (expected one of: {expected})"
    )]
    UnknownEnumValue {
        name: &'static str,
        value: String,
        expected: &'static str,
    },

    #[error("environment variable `{name}` is not a valid URL: {source}")]
    InvalidUrl {
        name: &'static str,
        #[source]
        source: url::ParseError,
    },

    #[error("provider id `{0}` is invalid: {1}")]
    InvalidProviderId(String, #[source] crate::ProviderIdError),

    #[error("role `{role}` references unknown provider `{provider}`")]
    UnknownProvider {
        role: &'static str,
        provider: ProviderId,
    },

    #[error(
        "provider kind `{kind}` is recognised in the schema but not yet supported by the env loader; \
         set `EURORA_LLM_KIND=openai` (default) or `openai_compatible`"
    )]
    KindNotYetWired { kind: &'static str },

    #[error(
        "`EURORA_LLM_BASE_URL` is required when `EURORA_LLM_KIND=openai_compatible` (point at e.g. \
         http://localhost:11434/v1 for an Ollama OpenAI shim)"
    )]
    OpenAiCompatibleBaseUrlRequired,

    #[error("base url `{url}` for provider `{provider}` must use http or https scheme")]
    InvalidScheme { provider: ProviderId, url: url::Url },
}

pub(crate) fn validate(config: &LlmConfig) -> Result<(), ConfigError> {
    check_role(config, "chat", &config.roles.chat.provider)?;
    check_role(config, "title", &config.roles.title.provider)?;
    if let Some(vision) = &config.roles.vision {
        check_role(config, "vision", &vision.provider)?;
    }

    for (id, provider) in &config.providers {
        if let crate::Provider::OpenAiCompatible { base_url, .. } = provider
            && base_url.scheme() != "http"
            && base_url.scheme() != "https"
        {
            return Err(ConfigError::InvalidScheme {
                provider: id.clone(),
                url: base_url.clone(),
            });
        }
    }

    Ok(())
}

fn check_role(
    config: &LlmConfig,
    role: &'static str,
    provider: &ProviderId,
) -> Result<(), ConfigError> {
    if !config.providers.contains_key(provider) {
        return Err(ConfigError::UnknownProvider {
            role,
            provider: provider.clone(),
        });
    }
    Ok(())
}
