//! Utility functions for LangChain.
//!
//! These functions do not depend on any other LangChain module.
//!
//! This module provides various utilities ported from `langchain_core/utils/`.

pub mod base;
pub mod env;
pub mod formatting;
pub mod function_calling;
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

// Re-export items from env.rs (mirrors langchain_core/utils/env.py)
pub use env::{EnvError, env_var_is_set, get_from_dict_or_env, get_from_env};

// Re-export items from uuid.rs (mirrors langchain_core/utils/uuid.py)
pub use uuid::uuid7;

// Re-export items from utils/utils.rs (mirrors langchain_core/utils/utils.py)
pub use base::{
    HttpStatusError, LC_AUTO_PREFIX, LC_ID_PREFIX, MockTime, NoDefault, SecretString, XorArgsError,
    build_model_kwargs, convert_to_secret_str, ensure_id, from_env, now_millis, now_secs,
    raise_for_status_with_text, secret_from_env, validate_xor_args,
};

// Re-export from other modules
pub use formatting::{FORMATTER, StrictFormatter, format_string};
pub use input::{get_bolded_text, get_color_mapping, get_colored_text, print_text};
pub use iter::{batch_iterate, tee};
pub use json::{parse_and_check_json_markdown, parse_json_markdown, parse_partial_json};
pub use json_schema::dereference_refs;
pub use merge::{merge_dicts, merge_lists, merge_obj};
pub use mustache::{MustacheValue, render as render_mustache};
pub use strings::{comma_list, sanitize_for_postgres, stringify_dict, stringify_value};
pub use usage::{UsageError, dict_int_op};
