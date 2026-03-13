mod chat_generation;
mod chat_result;
mod llm_result;
mod run_info;

use serde_json::Value;
use std::collections::HashMap;

use crate::utils::merge::merge_dicts;

pub use chat_generation::{ChatGeneration, ChatGenerationChunk, merge_chat_generation_chunks};
pub use chat_result::ChatResult;
pub use llm_result::{GenerationType, LLMResult};
pub use run_info::RunInfo;

fn merge_generation_info(
    left: Option<HashMap<String, Value>>,
    right: Option<HashMap<String, Value>>,
) -> Option<HashMap<String, Value>> {
    match (left, right) {
        (Some(left_map), Some(right_map)) => {
            let left_value = Value::Object(left_map.into_iter().collect());
            let right_value = Value::Object(right_map.into_iter().collect());
            match merge_dicts(left_value, vec![right_value]) {
                Ok(Value::Object(map)) => {
                    let result: HashMap<String, Value> = map.into_iter().collect();
                    if result.is_empty() {
                        None
                    } else {
                        Some(result)
                    }
                }
                _ => None,
            }
        }
        (Some(info), None) | (None, Some(info)) => Some(info),
        (None, None) => None,
    }
}
