//! Client-side parser for GLM-5.1's native XML tool-call format.
//!
//! GLM-family models served by vLLM expose tools through the OpenAI
//! Chat Completions wire format, but vLLM's tool-call parser does not
//! yet understand GLM-5.1's emission shape. With `tool_choice=auto` or
//! `tool_choice=required` the upstream parser silently eats the call;
//! with `tool_choice=none` the model emits its native format verbatim
//! in `delta.content`:
//!
//! ```xml
//! <tool_call>function_name
//!   <arg_key>param_one</arg_key>
//!   <arg_value>"hello"</arg_value>
//!   <arg_key>param_two</arg_key>
//!   <arg_value>42</arg_value>
//! </tool_call>
//! ```
//!
//! Zero-arg calls collapse to just `<tool_call>function_name</tool_call>`.
//! [`extract_tool_calls`] lifts these envelopes into structured
//! [`ToolCall`]s the existing dispatcher can run; [`strip_tool_call_envelopes`]
//! removes them from the persisted text. The orchestrator deliberately
//! requests `tool_choice=none` on the recovery round to force GLM into
//! this shape — see [`crate::agent_loop::try_round_with_retries`].

use std::sync::LazyLock;

use agent_chain::messages::ToolCall;
use regex::Regex;
use serde_json::{Map, Value};

/// Whole-envelope locator. `(?s)` lets `.` match newlines so a single
/// match covers a multi-line block. The `\b` after `tool_call` rules
/// out neighbours like `<tool_calls>`. The non-greedy body keeps
/// adjacent blocks from fusing into one match.
static TOOL_CALL_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<tool_call\b[^>]*>(.*?)</tool_call\s*>")
        .expect("glm tool_call block regex compiles")
});

/// Arg pair inside a block. Same `(?s)` + non-greedy semantics; the
/// `\s*` slack before each `>` tolerates `<arg_key >`-style whitespace.
static ARG_PAIR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<arg_key\s*>(.*?)</arg_key\s*>\s*<arg_value\s*>(.*?)</arg_value\s*>")
        .expect("glm arg pair regex compiles")
});

/// Cheap probe: does this content plausibly contain at least one
/// fully-closed GLM tool-call block? Used by the orchestrator before
/// paying for full extraction.
pub fn looks_like_glm_xml_tool_calls(content: &str) -> bool {
    content.contains("<tool_call") && TOOL_CALL_BLOCK_RE.is_match(content)
}

/// Extract every well-formed `<tool_call>` block from `content` and
/// return them as `ToolCall`s ready to feed into dispatch. Synthesises
/// `call_<uuid-v7>` ids since GLM does not provide them. Returns an
/// empty vec when there is nothing recoverable.
pub fn extract_tool_calls(content: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    for block in TOOL_CALL_BLOCK_RE.captures_iter(content) {
        let body = block.get(1).map(|m| m.as_str()).unwrap_or("");
        let Some(name) = extract_name(body) else {
            continue;
        };

        let mut args = Map::new();
        for pair in ARG_PAIR_RE.captures_iter(body) {
            let key = pair.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if key.is_empty() {
                continue;
            }
            let raw_value = pair.get(2).map(|m| m.as_str()).unwrap_or("");
            args.insert(key.to_string(), coerce_arg_value(raw_value));
        }

        calls.push(
            ToolCall::builder()
                .name(name)
                .args(Value::Object(args))
                .id(format!("call_{}", uuid::Uuid::now_v7().as_simple()))
                .build(),
        );
    }
    calls
}

/// Remove every `<tool_call>...</tool_call>` span from `content`. The
/// stripped string is what the user-facing chat persists; the lifted
/// calls go through the normal dispatch path.
pub fn strip_tool_call_envelopes(content: &str) -> String {
    TOOL_CALL_BLOCK_RE.replace_all(content, "").into_owned()
}

/// The body of a `<tool_call>` block looks like `NAME<arg_key>…` or
/// just `NAME` for zero-arg calls. Returns the trimmed function name,
/// or `None` if the body is empty (malformed; caller skips the block).
fn extract_name(body: &str) -> Option<String> {
    let head = match body.find("<arg_") {
        Some(idx) => &body[..idx],
        None => body,
    };
    let trimmed = head.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// GLM ships every arg as a raw string. The receiving tool schema
/// expects typed JSON. Try a strict JSON parse first; on failure fall
/// back to the verbatim string. `serde_json::from_str` consumes the
/// whole input or fails, so `"42abc"` correctly falls through.
fn coerce_arg_value(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Value::String(String::new());
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
        return parsed;
    }
    Value::String(raw.to_string())
}
