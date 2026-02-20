mod base;
pub mod chat_models;
mod fake;
mod fake_chat_models;
mod llms;
mod model_profile;
mod utils;

pub use base::{
    BaseLanguageModel, CustomGetTokenIds, LangSmithParams, LanguageModelConfig, LanguageModelInput,
    LanguageModelLike, LanguageModelOutput,
};

pub use chat_models::{
    AIMessageChunkStream, BaseChatModel, ChatChunk, ChatGenerationStream, ChatModelConfig,
    ChatModelRunnable, ChatStream, DisableStreaming, GenerateConfig, SimpleChatModel,
    StructuredOutputWithRaw, ToolChoice, ToolLike, agenerate_from_stream,
    cleanup_llm_representation, collect_and_merge_stream, extract_tool_name_from_schema,
    format_for_tracing, format_ls_structured_output, generate_from_stream,
    generate_response_from_error,
};

pub use crate::messages::UsageMetadata;

pub use llms::{
    BaseLLM, CacheValue, LLM, LLMConfig, LLMGenerateConfig, RunIdInput, aget_prompts_from_cache,
    aupdate_cache, create_base_retry, get_prompts_from_cache, get_run_ids_list, resolve_cache,
    save_llm, update_cache,
};

pub use fake::{FakeListLLM, FakeListLLMError, FakeStreamingListLLM};

pub use fake_chat_models::{
    FakeChatModel, FakeListChatModel, FakeListChatModelError, FakeMessagesListChatModel,
    GenericFakeChatModel, ParrotFakeChatModel,
};

pub use model_profile::{ModelProfile, ModelProfileRegistry};

pub use utils::{
    DataBlockFilter as OpenAiDataBlockFilter, ParsedDataUri, convert_legacy_v0_content_block_to_v1,
    convert_openai_format_to_data_block, estimate_token_count, get_token_ids_default,
    is_openai_data_block, normalize_messages, parse_data_uri, update_chunk_content_to_blocks,
    update_message_content_to_blocks,
};

pub type BoxedLanguageModelInput = Box<dyn std::any::Any + Send + Sync>;

pub type BoxedLanguageModelOutput = Box<dyn std::any::Any + Send + Sync>;
