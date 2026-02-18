#![allow(clippy::type_complexity, clippy::too_many_arguments)]
//! Agent Chain Core - A Rust implementation of LangChain core library.
//!
//! This crate provides:
//! - Message types for LLM threads (human, AI, system, tool)
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
//! - **Message layer** ([`messages`]): Message types for threads
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
pub mod indexing;
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
pub mod text_splitters;
pub mod tools;
pub mod tracers;
pub mod utils;
pub mod vectorstores;

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

    pub use crate::outputs::ChatResult;
}

pub use env::{VERSION, get_runtime_environment};

pub use error::{Error, Result};

pub use language_models::{
    AIMessageChunkStream,
    BaseChatModel,
    BaseLLM,
    BaseLanguageModel,
    ChatChunk,
    ChatGenerationStream,
    ChatModelConfig,
    ChatStream,
    DisableStreaming,
    FakeChatModel,
    FakeListChatModel,
    FakeListChatModelError,
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
    is_openai_data_block,
    parse_data_uri,
    update_cache,
};

pub use messages::{
    AIMessage, AnyMessage, BaseMessage, ContentPart, HasId, HumanMessage, ImageDetail, ImageSource,
    MergeableContent, MessageContent, SystemMessage, ToolCall, ToolMessage, convert_to_message,
    convert_to_messages,
};

pub use tools::{BaseTool, Tool, ToolDefinition};

pub use chat_history::{BaseChatMessageHistory, InMemoryChatMessageHistory};

pub use chat_sessions::ChatSession;

pub use chat_loaders::BaseChatLoader;

pub use caches::{BaseCache, CacheReturnValue, InMemoryCache};

pub use globals::{get_debug, get_llm_cache, get_verbose, set_debug, set_llm_cache, set_verbose};

pub use output_parsers::{
    BaseCumulativeTransformOutputParser, BaseLLMOutputParser, BaseOutputParser,
    BaseTransformOutputParser, CommaSeparatedListOutputParser, JsonOutputParser, ListOutputParser,
    MarkdownListOutputParser, NumberedListOutputParser, OutputParserError, ParseMatch,
    PydanticOutputParser, SimpleJsonOutputParser, StrOutputParser, XMLOutputParser, drop_last_n,
};

pub use outputs::{
    ChatGeneration, ChatGenerationChunk, ChatResult, Generation, GenerationChunk, GenerationType,
    LLMResult, RunInfo, merge_chat_generation_chunks,
};

pub use callbacks::{
    AsyncCallbackHandler, AsyncCallbackManager, AsyncCallbackManagerForChainRun,
    AsyncCallbackManagerForLLMRun, BaseCallbackHandler, BaseCallbackManager, CallbackManager,
    CallbackManagerForChainRun, CallbackManagerForLLMRun, Callbacks, StdOutCallbackHandler,
    StreamingStdOutCallbackHandler, UsageMetadataCallbackHandler,
};

pub use prompts::{
    AIMessagePromptTemplate, BaseChatPromptTemplate, BaseMessagePromptTemplate, BasePromptTemplate,
    ChatMessagePromptTemplate, ChatPromptTemplate, DictPromptTemplate,
    FewShotChatMessagePromptTemplate, FewShotPromptTemplate, FewShotPromptWithTemplates,
    HumanMessagePromptTemplate, ImagePromptTemplate, MessagesPlaceholder, PromptTemplate,
    PromptTemplateFormat, StringPromptTemplate, SystemMessagePromptTemplate, load_prompt,
};

pub use load::{
    ConstructorInfo, RevivedValue, Reviver, ReviverConfig, Serializable, Serialized,
    SerializedConstructor, SerializedNotImplemented, SerializedSecret, dumpd, dumps,
    load as load_json, loads,
};

pub use prompt_values::{
    ChatPromptValue, ChatPromptValueConcrete, ImageDetailLevel, ImagePromptValue, ImageURL,
    PromptValue, StringPromptValue,
};

pub use tracers::{
    AsyncBaseTracer, AsyncListener, AsyncRootListenersTracer, BaseTracer, ConsoleCallbackHandler,
    FunctionCallbackHandler, Listener, PassthroughStreamingHandler, RootListenersTracer, Run,
    RunCollectorCallbackHandler, RunEvent, RunType, SchemaFormat, StreamingCallbackHandler,
    TracerCore, TracerCoreConfig, TracerError,
};

pub use rate_limiters::{BaseRateLimiter, InMemoryRateLimiter, InMemoryRateLimiterConfig};

pub use agents::{AgentAction, AgentActionMessageLog, AgentFinish, AgentStep, ToolInput};

pub use document_loaders::{
    BaseBlobParser, BaseLoader as BaseDocumentLoader, BlobLoader, PathLike,
};

pub use documents::{
    BaseDocumentCompressor, BaseDocumentTransformer, BaseMedia, Blob, BlobBuilder, BlobData,
    Document,
};

pub use text_splitters::TextSplitter;

pub use retrievers::{BaseRetriever, LangSmithRetrieverParams, RetrieverInput, RetrieverOutput};

pub use stores::{
    BaseStore, InMemoryBaseStore, InMemoryByteStore, InMemoryStore, InvalidKeyException,
};

pub use runnables::{
    AddableDict, BaseStreamEvent, CUSTOM_EVENT_TYPE, ConfigOrList, CustomStreamEvent,
    DynRouterRunnable, DynRunnable, EventData, PickKeys, RouterInput, RouterRunnable, Runnable,
    RunnableAssign, RunnableAssignBuilder, RunnableBinding, RunnableConfig, RunnableEach,
    RunnableLambda, RunnableLambdaWithConfig, RunnableParallel, RunnablePassthrough, RunnablePick,
    RunnableRetry, RunnableSequence, RunnableSerializable, StandardStreamEvent, StreamEvent,
    coerce_to_runnable, ensure_config, get_config_list, graph_passthrough, merge_configs,
    patch_config, pipe, runnable_lambda, to_dyn,
};

pub use structured_query::{
    Comparator, Comparison, Expr, FilterDirective, FilterDirectiveEnum, Operation, Operator,
    OperatorOrComparator, StructuredQuery, Visitor,
};

pub use sys_info::{PackageInfo, SystemInfo, get_sys_info, get_sys_info_map, print_sys_info};

pub use async_trait::async_trait;

pub use embeddings::{DeterministicFakeEmbedding, Embeddings, FakeEmbeddings};

pub use vectorstores::{
    InMemoryVectorStore, SearchType, VectorStore, VectorStoreFactory, VectorStoreRetriever,
    VectorStoreRetrieverConfig, VectorStoreRetrieverExt, cosine_similarity,
    maximal_marginal_relevance,
};

pub use example_selectors::{
    BaseExampleSelector, LengthBasedExampleSelector, MaxMarginalRelevanceExampleSelector,
    SemanticSimilarityExampleSelector, sorted_values,
};
