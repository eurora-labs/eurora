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

pub mod agents;
pub mod api;
pub mod caches;
pub mod callbacks;
pub mod chat_history;
pub mod chat_loaders;
pub mod chat_sessions;
pub mod document_loaders;
pub mod documents;
pub mod embeddings;
pub mod env;
pub mod error;
pub mod example_selectors;
pub mod globals;
pub mod language_models;
pub mod load;
pub mod messages;
pub mod output_parsers;
pub mod outputs;
pub mod prompt_values;
pub mod prompts;
pub mod rate_limiters;
pub mod retrievers;
pub mod runnables;
pub mod stores;
pub mod structured_query;
pub mod sys_info;
pub mod tools;
pub mod tracers;
pub mod utils;
pub mod vectorstores;

// Keep chat_models as a backward-compatible re-export
pub mod chat_models {
    //! Re-export of language_models types for backward compatibility.
    //!
    //! This module re-exports types from [`language_models`] to maintain
    //! backward compatibility with code using the old `chat_models` module.
    //!
    //! New code should use [`language_models`] directly.

    pub use crate::language_models::{
        BaseChatModel, ChatChunk, ChatModelConfig, ChatStream, DisableStreaming, LangSmithParams,
        ToolChoice, UsageMetadata,
    };

    // Re-export ChatResult from outputs for backward compatibility
    pub use crate::outputs::ChatResult;
}

// Re-export env types
pub use env::{VERSION, get_runtime_environment};

// Re-export error types
pub use error::{Error, Result};

// Re-export core language model types
pub use language_models::{
    // Chat model types
    AIMessageChunkStream,
    BaseChatModel,
    // LLM types
    BaseLLM,
    // Base types
    BaseLanguageModel,
    ChatChunk,
    ChatGenerationStream,
    ChatModelConfig,
    ChatStream,
    DisableStreaming,
    FakeChatModel,
    FakeListChatModel,
    FakeListChatModelError,
    // Fake implementations for testing
    FakeListLLM,
    FakeListLLMError,
    FakeMessagesListChatModel,
    FakeStreamingListLLM,
    GenericFakeChatModel,
    LLM,
    LLMConfig,
    LangSmithParams,
    LanguageModelConfig,
    LanguageModelInput,
    LanguageModelOutput,

    // Model profile types
    ModelProfile,
    ModelProfileRegistry,

    OpenAiDataBlockFilter,
    ParrotFakeChatModel,

    ParsedDataUri,
    SimpleChatModel,
    ToolChoice,
    UsageMetadata,
    agenerate_from_stream,
    collect_and_merge_stream,
    generate_from_stream,

    get_prompts_from_cache,
    // Utility functions
    is_openai_data_block,
    parse_data_uri,
    update_cache,
};

// Re-export message types
pub use messages::{
    AIMessage, AnyMessage, BaseMessage, ContentPart, HasId, HumanMessage, ImageDetail, ImageSource,
    MergeableContent, MessageContent, SystemMessage, ToolCall, ToolMessage, convert_to_message,
    convert_to_messages,
};

// Re-export tool types
pub use tools::{BaseTool, Tool, ToolDefinition};

// Re-export chat history types
pub use chat_history::{BaseChatMessageHistory, InMemoryChatMessageHistory};

// Re-export chat session types
pub use chat_sessions::ChatSession;

// Re-export chat loader types
pub use chat_loaders::BaseChatLoader;

// Re-export cache types
pub use caches::{BaseCache, CacheReturnValue, InMemoryCache};

// Re-export global functions
pub use globals::{get_debug, get_llm_cache, get_verbose, set_debug, set_llm_cache, set_verbose};

// Re-export output parser types
pub use output_parsers::{
    BaseCumulativeTransformOutputParser, BaseLLMOutputParser, BaseOutputParser,
    BaseTransformOutputParser, CommaSeparatedListOutputParser, JsonOutputParser, ListOutputParser,
    MarkdownListOutputParser, NumberedListOutputParser, OutputParserError, ParseMatch,
    PydanticOutputParser, SimpleJsonOutputParser, StrOutputParser, XMLOutputParser, drop_last_n,
};

// Re-export output types
pub use outputs::{
    ChatGeneration, ChatGenerationChunk, ChatResult, Generation, GenerationChunk, GenerationType,
    LLMResult, RunInfo, merge_chat_generation_chunks,
};

// Re-export callback types
pub use callbacks::{
    AsyncCallbackHandler, AsyncCallbackManager, AsyncCallbackManagerForChainRun,
    AsyncCallbackManagerForLLMRun, BaseCallbackHandler, BaseCallbackManager, CallbackManager,
    CallbackManagerForChainRun, CallbackManagerForLLMRun, Callbacks, StdOutCallbackHandler,
    StreamingStdOutCallbackHandler, UsageMetadataCallbackHandler,
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

// Re-export prompt value types
pub use prompt_values::{
    ChatPromptValue, ChatPromptValueConcrete, ImageDetailLevel, ImagePromptValue, ImageURL,
    PromptValue, StringPromptValue,
};

// Re-export tracer types
pub use tracers::{
    AsyncBaseTracer, AsyncListener, AsyncRootListenersTracer, BaseTracer, ConsoleCallbackHandler,
    FunctionCallbackHandler, Listener, PassthroughStreamingHandler, RootListenersTracer, Run,
    RunCollectorCallbackHandler, RunEvent, RunType, SchemaFormat, StreamingCallbackHandler,
    TracerCore, TracerCoreConfig, TracerError,
};

// Re-export rate limiter types
pub use rate_limiters::{BaseRateLimiter, InMemoryRateLimiter, InMemoryRateLimiterConfig};

// Re-export agent types
pub use agents::{AgentAction, AgentActionMessageLog, AgentFinish, AgentStep, ToolInput};

// Re-export document loader types
pub use document_loaders::{
    BaseBlobParser, BaseLoader as BaseDocumentLoader, BlobLoader, PathLike,
};

// Re-export document types
pub use documents::{
    BaseDocumentCompressor, BaseDocumentTransformer, BaseMedia, Blob, BlobBuilder, BlobData,
    Document, FilterTransformer, FunctionTransformer,
};

// Re-export retriever types
pub use retrievers::{
    BaseRetriever, DynRetriever, FilterRetriever, LangSmithRetrieverParams, RetrieverInput,
    RetrieverOutput, SimpleRetriever,
};

// Re-export store types
pub use stores::{
    BaseStore, InMemoryBaseStore, InMemoryByteStore, InMemoryStore, InvalidKeyException,
};

// Re-export runnable types
pub use runnables::{
    AddableDict, BaseStreamEvent, CUSTOM_EVENT_TYPE, ConfigOrList, CustomStreamEvent,
    DynRouterRunnable, DynRunnable, EventData, PickKeys, RouterInput, RouterRunnable, Runnable,
    RunnableAssign, RunnableAssignBuilder, RunnableBinding, RunnableConfig, RunnableEach,
    RunnableLambda, RunnableLambdaWithConfig, RunnableParallel, RunnablePassthrough, RunnablePick,
    RunnableRetry, RunnableSequence, RunnableSerializable, StandardStreamEvent, StreamEvent,
    coerce_to_runnable, ensure_config, get_config_list, graph_passthrough, merge_configs,
    patch_config, pipe, runnable_lambda, to_dyn,
};

// Re-export structured query types
pub use structured_query::{
    Comparator, Comparison, Expr, FilterDirective, FilterDirectiveEnum, Operation, Operator,
    OperatorOrComparator, StructuredQuery, Visitor,
};

// Re-export sys_info types
pub use sys_info::{PackageInfo, SystemInfo, get_sys_info, get_sys_info_map, print_sys_info};

// Re-export async_trait for use in generated code
pub use async_trait::async_trait;

// Re-export embedding types
pub use embeddings::{DeterministicFakeEmbedding, Embeddings, FakeEmbeddings};

// Re-export vector store types
pub use vectorstores::{
    InMemoryVectorStore, SearchType, VectorStore, VectorStoreRetrieverConfig, cosine_similarity,
    maximal_marginal_relevance,
};

// Re-export example selector types
pub use example_selectors::{
    BaseExampleSelector, LengthBasedExampleSelector, MaxMarginalRelevanceExampleSelector,
    SemanticSimilarityExampleSelector, sorted_values,
};
