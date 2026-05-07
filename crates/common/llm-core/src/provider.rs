use std::collections::HashMap;
use std::fmt;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

// `Provider`, `GoogleCreds`, and `AwsCreds` carry [`SecretString`]s and so
// derive neither `Serialize` nor `Deserialize` themselves — `SecretString`
// intentionally refuses to serialize. Use [`crate::RedactedProvider`] for
// any over-the-wire view. Secret-free types in this module (`Roles`,
// `ModelRef`, `RequestOverrides`, `ProviderKind`) keep both traits so the
// redacted config can round-trip through JSON. `ProviderId` is a plain
// `String` alias; see its docs for the rationale.

/// A single named LLM provider.
///
/// The [`ProviderKind`] discriminant determines which fields are meaningful;
/// the variants intentionally expose provider-specific knobs (e.g. AWS region,
/// Google project) rather than collapsing everything into a generic struct,
/// because the parameter surface differs enough between providers that a
/// flat struct would force every consumer to handle "is this field meaningful
/// for this kind?" themselves.
///
/// `Provider` is constructed in code (by [`crate::from_env`]) rather than
/// deserialized, so it does not implement `Deserialize`. Use
/// [`crate::RedactedProvider`] when you need to ship a serialised view across
/// the wire — that one drops every secret-bearing field.
#[derive(Debug, Clone)]
pub enum Provider {
    OpenAI {
        api_key: SecretString,
        base_url: Option<Url>,
        organization: Option<String>,
    },
    Anthropic {
        api_key: SecretString,
        base_url: Option<Url>,
    },
    Google {
        credentials: GoogleCreds,
        project: Option<String>,
    },
    Bedrock {
        region: String,
        credentials: AwsCreds,
    },
    /// Any server speaking the OpenAI Chat Completions wire format. Used for
    /// Ollama (with its OpenAI shim), LM Studio, vLLM, llama.cpp's HTTP
    /// server, Groq, OpenRouter, etc.
    OpenAiCompatible {
        base_url: Url,
        api_key: Option<SecretString>,
        headers: HashMap<String, String>,
        overrides: RequestOverrides,
    },
}

impl Provider {
    /// Stable string discriminant for logging and the redacted view.
    pub fn kind(&self) -> ProviderKind {
        match self {
            Provider::OpenAI { .. } => ProviderKind::OpenAI,
            Provider::Anthropic { .. } => ProviderKind::Anthropic,
            Provider::Google { .. } => ProviderKind::Google,
            Provider::Bedrock { .. } => ProviderKind::Bedrock,
            Provider::OpenAiCompatible { .. } => ProviderKind::OpenAiCompatible,
        }
    }
}

/// Stable string discriminant for [`Provider`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Google,
    Bedrock,
    OpenAiCompatible,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::OpenAI => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Google => "google",
            ProviderKind::Bedrock => "bedrock",
            ProviderKind::OpenAiCompatible => "openai_compatible",
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Google Vertex / GenAI credentials. Adding a new arm to this enum is the
/// extension point for service-account auth or Workload Identity later.
#[derive(Debug, Clone)]
pub enum GoogleCreds {
    ApiKey {
        key: SecretString,
    },
    /// Path to a service-account JSON file on disk.
    ServiceAccount {
        path: String,
    },
}

/// AWS credentials for Bedrock. `Default` defers to the AWS SDK credential
/// chain (env, profile, IMDS); `Static` accepts explicit keys for the rare
/// case where they live outside the chain.
#[derive(Debug, Clone)]
pub enum AwsCreds {
    Default,
    Static {
        access_key_id: String,
        secret_access_key: SecretString,
        session_token: Option<String>,
    },
}

/// Request-body modifications applied to every LLM call routed through an
/// [`Provider::OpenAiCompatible`] provider. Useful for servers whose default
/// payload shape diverges from OpenAI's (e.g. vLLM extensions, non-standard
/// stop fields).
///
/// `force` is a JSON object merged into the outgoing request body. `strip`
/// is a list of top-level keys removed from the request body before sending.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestOverrides {
    #[serde(default)]
    pub force: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub strip: Vec<String>,
}

impl RequestOverrides {
    pub fn is_empty(&self) -> bool {
        self.force.is_empty() && self.strip.is_empty()
    }
}

/// Lookup key in [`crate::LlmConfig::providers`] and the reference field on
/// [`ModelRef`].
///
/// Aliased to [`String`] rather than a tuple-struct newtype because
/// `specta-typescript`'s map-key validator rejects newtypes as `HashMap`
/// keys, and the over-the-wire shape is shared with the desktop app via
/// taurpc/specta. Validation is exposed as the free function
/// [`validate_provider_id`] so a future config-file loader can call it from
/// its `Deserialize` impl — the env loader already only emits ids derived
/// from [`ProviderKind`], which are validator-clean by construction.
pub type ProviderId = String;

/// Validate a candidate provider id against the convention used in the env
/// loader and (eventually) any external loader.
///
/// Constraints: 1–64 chars, must start with `[a-z0-9]`, body may additionally
/// contain `_` and `-`. Lowercase ASCII only. Designed to keep ids safe to
/// embed in URLs, log lines, and TS object keys without further escaping.
pub fn validate_provider_id(value: &str) -> Result<(), ProviderIdError> {
    if value.is_empty() {
        return Err(ProviderIdError::Empty);
    }
    if value.len() > 64 {
        return Err(ProviderIdError::TooLong(value.to_string()));
    }
    let mut chars = value.chars();
    let first = chars.next().expect("non-empty");
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(ProviderIdError::InvalidChars(value.to_string()));
    }
    for ch in chars {
        let ok = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-';
        if !ok {
            return Err(ProviderIdError::InvalidChars(value.to_string()));
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderIdError {
    #[error("provider id is empty")]
    Empty,
    #[error("provider id `{0}` is longer than 64 characters")]
    TooLong(String),
    #[error(
        "provider id `{0}` must match [a-z0-9][a-z0-9_-]* (lowercase ASCII, starting with a letter or digit)"
    )]
    InvalidChars(String),
}

/// Pointer to a model owned by a specific provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ModelRef {
    pub provider: ProviderId,
    pub model: String,
}

/// Per-role model assignments. `chat` and `title` are mandatory; `vision` is
/// only required when the deployment expects to handle image-bearing
/// messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Roles {
    pub chat: ModelRef,
    pub title: ModelRef,
    pub vision: Option<ModelRef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_provider_id_accepts_lowercase_ascii() {
        validate_provider_id("openai").unwrap();
        validate_provider_id("openai-prod").unwrap();
        validate_provider_id("ollama_local").unwrap();
        validate_provider_id("p1").unwrap();
    }

    #[test]
    fn validate_provider_id_rejects_invalid_input() {
        assert!(matches!(
            validate_provider_id(""),
            Err(ProviderIdError::Empty)
        ));
        assert!(matches!(
            validate_provider_id("OpenAI"),
            Err(ProviderIdError::InvalidChars(_))
        ));
        assert!(matches!(
            validate_provider_id("-leading-dash"),
            Err(ProviderIdError::InvalidChars(_))
        ));
        assert!(matches!(
            validate_provider_id("with space"),
            Err(ProviderIdError::InvalidChars(_))
        ));
        assert!(matches!(
            validate_provider_id(&"a".repeat(65)),
            Err(ProviderIdError::TooLong(_))
        ));
    }

    #[test]
    fn provider_kind_str_passes_validation() {
        for kind in [
            ProviderKind::OpenAI,
            ProviderKind::Anthropic,
            ProviderKind::Google,
            ProviderKind::Bedrock,
            ProviderKind::OpenAiCompatible,
        ] {
            validate_provider_id(kind.as_str())
                .expect("ProviderKind::as_str values are validator-clean by construction");
        }
    }
}
