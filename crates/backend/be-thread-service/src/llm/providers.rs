//! Materialise [`agent_chain::BaseChatModel`] instances from a typed
//! [`llm_core::LlmConfig`].
//!
//! Provider selection lives entirely in `llm-core`; this module is the bridge
//! between that schema and the concrete `agent-chain` clients used at
//! runtime. Adding a new provider kind means adding an arm to
//! [`build_chat_model`] (and pulling in the relevant `agent-chain` provider
//! feature flag in `Cargo.toml`).
//!
//! `Anthropic`, `Google`, and `Bedrock` arms of [`llm_core::Provider`] are
//! valid in the schema but currently rejected here with
//! [`BuildError::KindNotYetWired`] — the env loader doesn't emit them today,
//! so this only fires for future config-file paths.
use std::sync::Arc;

use agent_chain::{BaseChatModel, BaseTool, openai::ChatOpenAI};
use llm_core::{LlmConfig, ModelRef, Provider, ProviderId};
use secrecy::ExposeSecret;

use crate::tools::firecrawl_tools;

/// Errors raised while turning [`LlmConfig`] into a concrete [`Providers`].
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("role `{role}` references unknown provider `{provider}`")]
    UnknownProvider {
        role: &'static str,
        provider: ProviderId,
    },

    #[error(
        "provider kind `{kind}` is recognised in the schema but the runtime client is not yet \
         wired in be-thread-service"
    )]
    KindNotYetWired { kind: &'static str },

    #[error(
        "provider `{provider}` carries non-empty `headers` or `overrides`, which are not yet \
         honoured by the underlying agent-chain OpenAI client"
    )]
    UnsupportedFeature { provider: ProviderId },
}

pub struct Providers {
    pub chat: Arc<dyn BaseChatModel + Send + Sync>,
    pub title: Arc<dyn BaseChatModel + Send + Sync>,
    pub vision: Option<VisionConfig>,
}

pub struct VisionConfig {
    pub model: Arc<dyn BaseChatModel + Send + Sync>,
    pub default_tools: Vec<Arc<dyn BaseTool>>,
}

/// Construct provider clients for each role in [`LlmConfig::roles`].
///
/// Vision tools are attached when the deployment has both a vision role and
/// a `FIRECRAWL_API_KEY` env var set — without the key the tools would fail
/// on every call, so we'd rather hand the model a tool-less context than
/// pretend the tools work.
pub fn build_providers(cfg: &LlmConfig) -> Result<Providers, BuildError> {
    let chat = build_chat_model(cfg, "chat", &cfg.roles.chat)?;
    let title = build_chat_model(cfg, "title", &cfg.roles.title)?;
    let vision = match cfg.roles.vision.as_ref() {
        Some(role) => {
            let model = build_chat_model(cfg, "vision", role)?;
            let default_tools = if std::env::var("FIRECRAWL_API_KEY").is_ok_and(|v| !v.is_empty()) {
                firecrawl_tools()
            } else {
                tracing::info!(
                    "Vision role configured but FIRECRAWL_API_KEY is unset — \
                     skipping firecrawl tool registration"
                );
                Vec::new()
            };
            Some(VisionConfig {
                model,
                default_tools,
            })
        }
        None => None,
    };

    Ok(Providers {
        chat,
        title,
        vision,
    })
}

fn build_chat_model(
    cfg: &LlmConfig,
    role: &'static str,
    model_ref: &ModelRef,
) -> Result<Arc<dyn BaseChatModel + Send + Sync>, BuildError> {
    let provider =
        cfg.providers
            .get(&model_ref.provider)
            .ok_or_else(|| BuildError::UnknownProvider {
                role,
                provider: model_ref.provider.clone(),
            })?;

    match provider {
        Provider::OpenAI {
            api_key,
            base_url,
            organization,
        } => {
            let model = ChatOpenAI::builder()
                .model(model_ref.model.clone())
                .api_key(api_key.expose_secret().to_string())
                .maybe_api_base(base_url.as_ref().map(|u| u.as_str().to_string()))
                .maybe_organization(organization.clone())
                .build();
            Ok(Arc::new(model))
        }
        Provider::OpenAiCompatible {
            base_url,
            api_key,
            headers,
            overrides,
        } => {
            if !headers.is_empty() || !overrides.is_empty() {
                return Err(BuildError::UnsupportedFeature {
                    provider: model_ref.provider.clone(),
                });
            }
            // Pass an explicit placeholder when no key is configured: the
            // alternative is `ChatOpenAI` falling back to `OPENAI_API_KEY`
            // from the environment, which would silently send the operator's
            // OpenAI key to whatever local server they configured.
            let api_key_value = api_key
                .as_ref()
                .map(|k| k.expose_secret().to_string())
                .unwrap_or_else(|| "not-needed".to_string());
            let model = ChatOpenAI::builder()
                .model(model_ref.model.clone())
                .api_base(base_url.as_str().to_string())
                .api_key(api_key_value)
                .build();
            Ok(Arc::new(model))
        }
        Provider::Anthropic { .. } => Err(BuildError::KindNotYetWired { kind: "anthropic" }),
        Provider::Google { .. } => Err(BuildError::KindNotYetWired { kind: "google" }),
        Provider::Bedrock { .. } => Err(BuildError::KindNotYetWired { kind: "bedrock" }),
    }
}
