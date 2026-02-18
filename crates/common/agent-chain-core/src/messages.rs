//! Message types for LLM interactions.
//!
//! This module provides message types for different roles (human, AI, system, tool)
//! as well as types for tool calls. The structure mirrors the Python langchain_core.messages
//! module.
//!
//! # Multimodal Support
//!
//! The [`HumanMessage`] type supports multimodal content including text and images.
//! Images can be provided as URLs or base64-encoded data.
//!
//! ```ignore
//! use agent_chain_core::messages::{HumanMessage, ContentPart, ImageSource};
//!
//! // Simple text message
//! let msg = HumanMessage::builder().content("Hello!").build();
//!
//! // Message with image from URL
//! let msg = HumanMessage::with_content(vec![
//!     ContentPart::Text { text: "What's in this image?".into() },
//!     ContentPart::Image {
//!         source: ImageSource::Url {
//!             url: "https://example.com/image.jpg".into(),
//!         },
//!         detail: None,
//!     },
//! ]);
//!
//! // Message with base64-encoded image
//! let msg = HumanMessage::with_content(vec![
//!     ContentPart::Text { text: "Describe this image".into() },
//!     ContentPart::Image {
//!         source: ImageSource::Base64 {
//!             media_type: "image/jpeg".into(),
//!             data: base64_image_data,
//!         },
//!         detail: Some(ImageDetail::High),
//!     },
//! ]);
//! ```

mod ai;
mod base;
pub mod block_translators;
mod chat;
pub mod content;
mod function;
mod human;
mod modifier;
mod system;
mod tool;
pub(crate) mod utils;

pub use ai::{
    AIMessage, AIMessageChunk, ChunkPosition, InputTokenDetails, OutputTokenDetails, UsageMetadata,
    add_ai_message_chunks, add_usage, backwards_compat_tool_calls, subtract_usage,
};

pub use base::{
    BaseMessage, BaseMessageChunk, HasId, MergeableContent,
    extract_reasoning_from_additional_kwargs, get_bolded_text, get_msg_title_repr,
    is_interactive_env, merge_content, merge_content_complex, merge_content_vec,
    message_to_dict as base_message_to_dict, messages_to_dict as base_messages_to_dict,
};

pub use chat::{ChatMessage, ChatMessageChunk};

pub use content::{
    Annotation,
    AudioContentBlock,
    BlockIndex,
    Citation,
    ContentBlock,
    ContentPart,
    DataContentBlock,
    FileContentBlock,
    ImageContentBlock,
    ImageDetail,
    ImageSource,
    InvalidToolCallBlock,
    KNOWN_BLOCK_TYPES,
    MessageContent,
    NonStandardAnnotation,
    NonStandardContentBlock,
    PlainTextBlockConfig,
    PlainTextContentBlock,
    ReasoningContentBlock,
    ServerToolCall,
    ServerToolCallChunk,
    ServerToolResult,
    ServerToolStatus,
    TextContentBlock,
    ToolCallBlock,
    ToolCallChunkBlock,
    ToolContentBlock,
    VideoContentBlock,
    create_audio_block,
    create_citation,
    create_file_block,
    create_image_block,
    create_non_standard_block,
    create_plaintext_block,
    create_reasoning_block,
    create_text_block,
    create_tool_call,
    create_video_block,
    get_data_content_block_types,
    is_data_content_block,
};

pub use function::{FunctionMessage, FunctionMessageChunk};

pub use human::{HumanMessage, HumanMessageChunk};

pub use modifier::RemoveMessage;

pub use system::{SystemMessage, SystemMessageChunk};

pub use tool::{
    InvalidToolCall, ToolCall, ToolCallChunk, ToolMessage, ToolMessageChunk, ToolOutputMixin,
    ToolStatus, default_tool_chunk_parser, default_tool_parser, invalid_tool_call, tool_call,
    tool_call_chunk,
};

pub use utils::{
    AnyMessage, CountTokensConfig, ExcludeToolCalls, MessageLikeRepresentation, TextFormat,
    TrimMessagesConfig, TrimStrategy, convert_to_message, convert_to_messages,
    convert_to_openai_messages, count_tokens_approximately, filter_messages,
    filter_messages_runnable, get_buffer_string, merge_message_runs, merge_message_runs_runnable,
    message_chunk_to_message, message_from_dict, message_to_dict, messages_from_dict,
    messages_to_dict, trim_messages, trim_messages_runnable,
};
