//! Agent Chain Core - A Rust implementation of LangChain core library.
//!
//! This crate provides:
//! - Message types for LLM conversations (human, AI, system, tool)
//! - Tool trait and `#[tool]` macro for function calling
//! - Chat model abstractions
//! - Callback system for tracking and monitoring operations
//! - Prompt templates for flexible prompt construction
//! - Support for multiple providers (Anthropic, OpenAI, etc.)
//!
//! # Architecture
//!
//! The architecture follows LangChain's pattern:
//!
//! - **Core layer** ([`chat_models`]): Base `ChatModel` trait that all providers implement
//! - **Message layer** ([`messages`]): Message types for conversations
//! - **Prompts layer** ([`prompts`]): Prompt templates for constructing prompts
//! - **Tools layer** ([`tools`]): Tool definitions and the `#[tool]` macro
//! - **Callbacks layer** ([`callbacks`]): Callback handlers and managers for monitoring
//!
//! # Feature Flags
//!
//! - `default`: Includes all providers
//! - `specta`: Specta derive support

pub mod api;
pub mod callbacks;
pub mod chat_models;
pub mod error;
pub mod load;
pub mod messages;
pub mod outputs;
pub mod prompts;
pub mod runnables;
pub mod tools;
pub mod tracers;
pub mod utils;

// Re-export error types
pub use error::{Error, Result};

// Re-export core chat model types
pub use chat_models::{
    BoundChatModel, ChatChunk, ChatModel, ChatModelExt, ChatResult, ChatResultMetadata, ChatStream,
    DynBoundChatModel, DynChatModelExt, LangSmithParams, ToolChoice, UsageMetadata,
};

// Re-export message types
pub use messages::{
    AIMessage, AnyMessage, BaseMessage, ContentPart, HasId, HumanMessage, ImageDetail, ImageSource,
    MessageContent, SystemMessage, ToolCall, ToolMessage,
};

// Re-export tool types
pub use tools::{Tool, ToolDefinition, tool};

// Re-export output types
pub use outputs::{
    ChatGeneration, ChatGenerationChunk, ChatResult as OutputChatResult, Generation,
    GenerationChunk, GenerationType, LLMResult, RunInfo, merge_chat_generation_chunks,
};

// Re-export callback types
pub use callbacks::{
    AsyncCallbackHandler, AsyncCallbackManager, AsyncCallbackManagerForChainRun,
    AsyncCallbackManagerForLLMRun, BaseCallbackHandler, BaseCallbackManager, CallbackManager,
    CallbackManagerForChainRun, CallbackManagerForLLMRun, Callbacks, StdOutCallbackHandler,
    StreamingStdOutCallbackHandler, UsageMetadataCallbackHandler, add_usage,
};

// Re-export prompt types
pub use prompts::{
    AIMessagePromptTemplate, BaseChatPromptTemplate, BaseMessagePromptTemplate, BasePromptTemplate,
    ChatMessagePromptTemplate, ChatPromptTemplate, DictPromptTemplate,
    FewShotChatMessagePromptTemplate, FewShotPromptTemplate, FewShotPromptWithTemplates,
    HumanMessagePromptTemplate, ImagePromptTemplate, MessagesPlaceholder, PromptTemplate,
    PromptTemplateFormat, StringPromptTemplate, SystemMessagePromptTemplate, load_prompt,
};

// Re-export load types
pub use load::{
    ConstructorInfo, RevivedValue, Reviver, ReviverConfig, Serializable, Serialized,
    SerializedConstructor, SerializedNotImplemented, SerializedSecret, dumpd, dumps,
    load as load_json, loads,
};

// Re-export tracer types
pub use tracers::{
    AsyncBaseTracer, AsyncListener, AsyncRootListenersTracer, BaseTracer, ConsoleCallbackHandler,
    FunctionCallbackHandler, Listener, PassthroughStreamingHandler, RootListenersTracer, Run,
    RunCollectorCallbackHandler, RunEvent, RunType, SchemaFormat, StreamingCallbackHandler,
    TracerCore, TracerCoreConfig, TracerError,
};

// Re-export runnable types
pub use runnables::{
    AddableDict, BaseStreamEvent, CUSTOM_EVENT_TYPE, ConfigOrList, CustomStreamEvent,
    DynRouterRunnable, DynRunnable, EventData, RouterInput, RouterRunnable, Runnable,
    RunnableBinding, RunnableConfig, RunnableEach, RunnableLambda, RunnableParallel,
    RunnablePassthrough, RunnableRetry, RunnableSequence, RunnableSerializable,
    StandardStreamEvent, StreamEvent, coerce_to_runnable, ensure_config, get_config_list,
    merge_configs, patch_config, pipe, runnable_lambda, to_dyn,
};

// Re-export async_trait for use in generated code
pub use async_trait::async_trait;
