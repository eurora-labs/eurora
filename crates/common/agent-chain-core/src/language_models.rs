//! Core language model abstractions.
//!
//! LangChain has two main classes to work with language models: chat models and
//! "old-fashioned" LLMs (string-in, string-out).
//!
//! # Chat models
//!
//! Language models that use a sequence of messages as inputs and return chat messages
//! as outputs (as opposed to using plain text).
//!
//! Chat models support the assignment of distinct roles to thread messages, helping
//! to distinguish messages from the AI, users, and instructions such as system messages.
//!
//! The key abstraction for chat models is [`BaseChatModel`]. Implementations should
//! implement this trait.
//!
//! # LLMs (legacy)
//!
//! Language models that take a string as input and return a string.
//!
//! These are traditionally older models (newer models generally are chat models).
//!
//! Although the underlying models are string in, string out, the LangChain wrappers also
//! allow these models to take messages as input. This gives them the same interface as
//! chat models. When messages are passed in as input, they will be formatted into a string
//! under the hood before being passed to the underlying model.
//!
//! # Example
//!
//! ```rust,ignore
//! use agent_chain_core::language_models::{BaseChatModel, ChatResult};
//! use agent_chain_core::messages::{BaseMessage, HumanMessage};
//!
//! async fn chat_with_model<M: BaseChatModel>(model: &M) -> Result<ChatResult, agent_chain_core::error::Error> {
//!     let messages = vec![
//!         BaseMessage::Human(HumanMessage::builder().content("Hello, how are you?").build()),
//!     ];
//!     model.generate(messages, GenerateConfig::default()).await
//! }
//! ```

mod base;
pub mod chat_models;
mod fake;
mod fake_chat_models;
mod llms;
mod model_profile;
mod utils;

// Re-export base types
pub use base::{
    BaseLanguageModel, CustomGetTokenIds, LangSmithParams, LanguageModelConfig, LanguageModelInput,
    LanguageModelLike, LanguageModelOutput,
};

// Re-export chat model types
pub use chat_models::{
    AIMessageChunkStream, BaseChatModel, ChatChunk, ChatGenerationStream, ChatModelConfig,
    ChatModelRunnable, ChatStream, DisableStreaming, GenerateConfig, SimpleChatModel,
    StructuredOutputWithRaw, ToolChoice, ToolLike, agenerate_from_stream,
    cleanup_llm_representation, collect_and_merge_stream, extract_tool_name_from_schema,
    format_for_tracing, format_ls_structured_output, generate_from_stream,
    generate_response_from_error,
};

// Re-export UsageMetadata from messages (where it's canonically defined)
pub use crate::messages::UsageMetadata;

// Re-export LLM types
pub use llms::{
    BaseLLM, CacheValue, LLM, LLMConfig, LLMGenerateConfig, RunIdInput, aget_prompts_from_cache,
    aupdate_cache, create_base_retry, get_prompts_from_cache, get_run_ids_list, resolve_cache,
    save_llm, update_cache,
};

// Re-export fake implementations for testing
pub use fake::{FakeListLLM, FakeListLLMError, FakeStreamingListLLM};

pub use fake_chat_models::{
    FakeChatModel, FakeListChatModel, FakeListChatModelError, FakeMessagesListChatModel,
    GenericFakeChatModel, ParrotFakeChatModel,
};

// Re-export model profile types
pub use model_profile::{ModelProfile, ModelProfileRegistry};

// Re-export utility functions
pub use utils::{
    DataBlockFilter as OpenAiDataBlockFilter, ParsedDataUri, convert_legacy_v0_content_block_to_v1,
    convert_openai_format_to_data_block, estimate_token_count, get_token_ids_default,
    is_openai_data_block, normalize_messages, parse_data_uri, update_chunk_content_to_blocks,
    update_message_content_to_blocks,
};

/// Type alias for a boxed language model input.
pub type BoxedLanguageModelInput = Box<dyn std::any::Any + Send + Sync>;

/// Type alias for a boxed language model output.
pub type BoxedLanguageModelOutput = Box<dyn std::any::Any + Send + Sync>;
