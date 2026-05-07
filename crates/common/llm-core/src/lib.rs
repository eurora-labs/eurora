//! Shared LLM provider configuration.
//!
//! `llm-core` defines the typed schema for picking an LLM provider and the
//! per-role models the backend should use. It is intentionally a pure
//! configuration crate: it does not perform HTTP requests, build clients, or
//! depend on any provider SDK. The `agent-chain` family of crates owns the
//! actual `BaseChatModel` implementations; this crate decides which ones to
//! instantiate and with what parameters.
//!
//! # Resolution
//!
//! The only loader currently exposed is [`LlmConfig::from_env`]. The full
//! [`Provider`] enum models OpenAI, Anthropic, Google, Bedrock, and a generic
//! OpenAI-compatible kind so that adding new providers later is a code change
//! to the consumers of this crate (e.g. `be-thread-service`) rather than a
//! schema change. The env loader currently only emits `OpenAI` or
//! `OpenAiCompatible` — the schema is forward-compatible with the rest.
//!
//! # Secrets
//!
//! API keys are wrapped in [`secrecy::SecretString`] so they don't appear in
//! logs or `Debug` output. The redacted view ([`RedactedLlmConfig`]) is what
//! you serve over HTTP / hand to the desktop app — it carries provider names,
//! models and base URLs but not key material.

mod load;
mod provider;
mod redacted;
mod validate;

pub use load::{ConfigSource, from_env};
pub use provider::{
    AwsCreds, GoogleCreds, ModelRef, Provider, ProviderId, ProviderIdError, ProviderKind,
    RequestOverrides, Roles, validate_provider_id,
};
pub use redacted::{RedactedLlmConfig, RedactedProvider};
pub use validate::ConfigError;

use std::collections::HashMap;

/// Top-level LLM configuration: a set of named providers plus the model
/// assignments for each role the backend supports.
///
/// Constructed by [`from_env`]; not deserialized directly because [`Provider`]
/// holds [`secrecy::SecretString`] values that intentionally refuse to
/// serialize. For the over-the-wire view see [`RedactedLlmConfig`].
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub providers: HashMap<ProviderId, Provider>,
    pub roles: Roles,
}

impl LlmConfig {
    /// Load configuration from environment variables.
    ///
    /// See [`from_env`] for the full set of variables understood. The returned
    /// [`ConfigSource`] reports which path was taken (currently only
    /// [`ConfigSource::Env`]) and is useful in startup logging.
    pub fn from_env() -> Result<(Self, ConfigSource), ConfigError> {
        load::from_env()
    }

    /// Strip secrets and return a view suitable for serving over the wire or
    /// printing at startup.
    pub fn redacted(&self) -> RedactedLlmConfig {
        RedactedLlmConfig::from(self)
    }
}
