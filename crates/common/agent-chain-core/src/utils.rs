//! Utility functions for LangChain.
//!
//! These functions do not depend on any other LangChain module.
//!
//! This module provides various utilities ported from `langchain_core/utils/`.

pub mod env;
pub mod formatting;
pub mod html;
pub mod input;
pub mod interactive_env;
pub mod iter;
pub mod json;
pub mod json_schema;
pub mod merge;
pub mod mustache;
pub mod strings;
pub mod usage;
pub mod uuid;

// Re-export commonly used items at the utils level
pub use env::{
    SecretString, env_var_is_set, from_env, get_from_dict_or_env, get_from_env, secret_from_env,
};
pub use formatting::{FORMATTER, StrictFormatter, format_string};
pub use input::{get_bolded_text, get_color_mapping, get_colored_text, print_text};
pub use iter::{batch_iterate, tee};
pub use json::{parse_and_check_json_markdown, parse_json_markdown, parse_partial_json};
pub use json_schema::{dereference_refs, remove_titles};
pub use merge::{merge_dicts, merge_lists, merge_obj};
pub use mustache::{MustacheValue, render as render_mustache};
pub use strings::{comma_list, sanitize_for_postgres, stringify_dict, stringify_value};
pub use usage::{
    UsageValue, dict_int_add, dict_int_add_json, dict_int_op, dict_int_op_json, dict_int_sub,
    dict_int_sub_floor_json,
};
pub use uuid::{LC_AUTO_PREFIX, LC_ID_PREFIX, ensure_id, uuid7};
