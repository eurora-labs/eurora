//! Integration tests for the env loader.
//!
//! These tests mutate process-wide environment state, so they all run in
//! `#[serial_test]`-style sequence by sharing a single mutex. We avoid pulling
//! in `serial_test` as a dependency — the lock here is enough.

use std::sync::Mutex;

use llm_core::{ConfigError, LlmConfig, Provider};
use secrecy::ExposeSecret;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const ALL_VARS: &[&str] = &[
    "EURORA_LLM_KIND",
    "OPENAI_API_KEY",
    "EURORA_OPENAI_ORG",
    "EURORA_LLM_BASE_URL",
    "EURORA_LLM_API_KEY",
    "EURORA_CHAT_MODEL",
    "EURORA_TITLE_MODEL",
    "EURORA_VISION_MODEL",
];

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    // A panicking test poisons the mutex, but each test resets the env at
    // the top — we don't carry any shared state between tests, so it's safe
    // to recover.
    match ENV_LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn clear_env() {
    for var in ALL_VARS {
        // SAFETY: tests are serialized via `ENV_LOCK`, so no other thread is
        // reading the environment concurrently.
        unsafe { std::env::remove_var(var) };
    }
}

fn set(name: &str, value: &str) {
    // SAFETY: see `clear_env`.
    unsafe { std::env::set_var(name, value) };
}

#[test]
fn defaults_to_openai_when_kind_unset() {
    let _g = env_lock();
    clear_env();
    set("OPENAI_API_KEY", "sk-test");
    set("EURORA_CHAT_MODEL", "gpt-4o-mini");

    let (config, _) = LlmConfig::from_env().expect("loads");
    assert_eq!(config.providers.len(), 1);
    let provider = config
        .providers
        .values()
        .next()
        .expect("exactly one provider");
    let Provider::OpenAI {
        api_key,
        base_url,
        organization,
    } = provider
    else {
        panic!("expected OpenAI provider, got {:?}", provider.kind());
    };
    assert_eq!(api_key.expose_secret(), "sk-test");
    assert!(base_url.is_none());
    assert!(organization.is_none());

    assert_eq!(config.roles.chat.model, "gpt-4o-mini");
    assert_eq!(config.roles.title.model, "gpt-4o-mini");
    assert!(config.roles.vision.is_none());
}

#[test]
fn title_falls_back_to_chat_model() {
    let _g = env_lock();
    clear_env();
    set("OPENAI_API_KEY", "sk-test");
    set("EURORA_CHAT_MODEL", "gpt-4o");
    set("EURORA_TITLE_MODEL", "gpt-4o-mini");

    let (config, _) = LlmConfig::from_env().expect("loads");
    assert_eq!(config.roles.chat.model, "gpt-4o");
    assert_eq!(config.roles.title.model, "gpt-4o-mini");
}

#[test]
fn vision_role_is_emitted_when_set() {
    let _g = env_lock();
    clear_env();
    set("OPENAI_API_KEY", "sk-test");
    set("EURORA_CHAT_MODEL", "gpt-4o");
    set("EURORA_VISION_MODEL", "gpt-4o");

    let (config, _) = LlmConfig::from_env().expect("loads");
    let vision = config.roles.vision.as_ref().expect("vision role present");
    assert_eq!(vision.model, "gpt-4o");
    assert_eq!(vision.provider, config.roles.chat.provider);
}

#[test]
fn openai_compatible_requires_base_url() {
    let _g = env_lock();
    clear_env();
    set("EURORA_LLM_KIND", "openai_compatible");
    set("EURORA_CHAT_MODEL", "llama3.2");

    let err = LlmConfig::from_env().expect_err("missing base url");
    assert!(matches!(err, ConfigError::OpenAiCompatibleBaseUrlRequired));
}

#[test]
fn openai_compatible_accepts_keyless_local_server() {
    let _g = env_lock();
    clear_env();
    set("EURORA_LLM_KIND", "openai_compatible");
    set("EURORA_LLM_BASE_URL", "http://localhost:11434/v1");
    set("EURORA_CHAT_MODEL", "llama3.2");

    let (config, _) = LlmConfig::from_env().expect("loads");
    let Provider::OpenAiCompatible {
        base_url, api_key, ..
    } = config
        .providers
        .values()
        .next()
        .expect("exactly one provider")
    else {
        panic!("expected OpenAiCompatible");
    };
    assert_eq!(base_url.as_str(), "http://localhost:11434/v1");
    assert!(api_key.is_none());
}

#[test]
fn missing_chat_model_is_an_error() {
    let _g = env_lock();
    clear_env();
    set("OPENAI_API_KEY", "sk-test");

    let err = LlmConfig::from_env().expect_err("no chat model");
    assert!(matches!(err, ConfigError::MissingEnv("EURORA_CHAT_MODEL")));
}

#[test]
fn missing_openai_key_is_an_error() {
    let _g = env_lock();
    clear_env();
    set("EURORA_CHAT_MODEL", "gpt-4o-mini");

    let err = LlmConfig::from_env().expect_err("no api key");
    assert!(matches!(err, ConfigError::MissingEnv("OPENAI_API_KEY")));
}

#[test]
fn unknown_kind_lists_supported_values() {
    let _g = env_lock();
    clear_env();
    set("EURORA_LLM_KIND", "magic");
    set("EURORA_CHAT_MODEL", "x");

    let err = LlmConfig::from_env().expect_err("bad kind");
    let ConfigError::UnknownEnumValue { name, value, .. } = err else {
        panic!("expected UnknownEnumValue, got {err:?}");
    };
    assert_eq!(name, "EURORA_LLM_KIND");
    assert_eq!(value, "magic");
}

#[test]
fn anthropic_kind_is_recognised_but_not_yet_wired() {
    let _g = env_lock();
    clear_env();
    set("EURORA_LLM_KIND", "anthropic");
    set("EURORA_CHAT_MODEL", "claude-sonnet-4-5");

    let err = LlmConfig::from_env().expect_err("not yet wired");
    assert!(matches!(
        err,
        ConfigError::KindNotYetWired { kind: "anthropic" }
    ));
}

#[test]
fn redacted_view_omits_secrets() {
    let _g = env_lock();
    clear_env();
    set("OPENAI_API_KEY", "super-secret-key-xyz");
    set("EURORA_CHAT_MODEL", "gpt-4o-mini");

    let (config, _) = LlmConfig::from_env().expect("loads");
    let redacted = config.redacted();

    let json = serde_json::to_string(&redacted).expect("serializes");
    assert!(
        !json.contains("super-secret-key-xyz"),
        "redacted view leaked the API key: {json}"
    );
    assert!(json.contains("\"has_api_key\":true"));
    assert!(json.contains("\"kind\":\"openai\""));
    assert_eq!(redacted.providers.len(), 1);
    assert_eq!(redacted.roles.chat.model, "gpt-4o-mini");
    assert_eq!(redacted.roles.chat.provider.as_str(), "openai");
}
