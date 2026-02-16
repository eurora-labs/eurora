//! Output classes.
//!
//! Used to represent the output of a language model call and the output of a chat.
//!
//! The top container for information is the `LLMResult` object. `LLMResult` is used by both
//! chat models and LLMs. This object contains the output of the language model and any
//! additional information that the model provider wants to return.
//!
//! When invoking models via the standard runnable methods (e.g. invoke, batch, etc.):
//!
//! - Chat models will return `AIMessage` objects.
//! - LLMs will return regular text strings.
//!
//! In addition, users can access the raw output of either LLMs or chat models via
//! callbacks. The `on_chat_model_end` and `on_llm_end` callbacks will return an
//! LLMResult object containing the generated outputs and any additional information
//! returned by the model provider.
//!
//! In general, if information is already available in the AIMessage object, it is
//! recommended to access it from there rather than from the `LLMResult` object.
//!
//! Mirrors `langchain_core.outputs`.

mod chat_generation;
mod chat_result;
mod generation;
mod llm_result;
mod run_info;

// Re-export from generation
pub use generation::{Generation, GenerationChunk, merge_generation_chunks};

// Re-export from run_info
pub use run_info::RunInfo;

// Re-export from chat_generation
pub use chat_generation::{ChatGeneration, ChatGenerationChunk, merge_chat_generation_chunks};

// Re-export from chat_result
pub use chat_result::ChatResult;

// Re-export from llm_result
pub use llm_result::{GenerationType, LLMResult};
