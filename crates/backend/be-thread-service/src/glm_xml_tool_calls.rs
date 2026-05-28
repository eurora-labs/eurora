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

/// Locator for the *start* of a `<tool_call>` opener. Shares the
/// `<tool_call\b` boundary with [`TOOL_CALL_BLOCK_RE`] so the streaming
/// filter and the block stripper agree on what counts as an opener —
/// neither treats `<tool_calls>` (plural) as one. Used only by
/// [`ToolCallStreamFilter`] to find a dangling, not-yet-closed opener.
static TOOL_CALL_OPEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<tool_call\b").expect("glm tool_call open regex compiles"));

/// The opener marker, used for the trailing-partial-prefix holdback.
const TOOL_CALL_OPEN_MARKER: &str = "<tool_call";

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

/// Streaming suppressor for GLM `<tool_call>` envelopes.
///
/// GLM-5.1 emits its native `<tool_call>…</tool_call>` markup inline in
/// the content channel (see module docs). Streamed verbatim to a UI,
/// the raw markup flashes by before the post-hoc
/// [`strip_tool_call_envelopes`] pass removes it from the persisted
/// message — so the live view and the reloaded view disagree.
///
/// This filter closes that gap. It is fed the *cumulative* round
/// content on every chunk (not the per-chunk delta) and returns only
/// the newly-safe text to forward. It holds back:
///
/// - any region from a dangling `<tool_call` opener whose
///   `</tool_call>` has not yet arrived (it may still be removed), and
/// - a trailing partial that could be the start of `<tool_call` (so a
///   `<tool_c` split across chunk boundaries is never leaked).
///
/// Suppression runs the *same* [`strip_tool_call_envelopes`] the
/// persisted message uses, so the forwarded stream and the stored
/// content are identical by construction — they cannot drift even if
/// the envelope grammar changes. This mirrors LangChain's
/// `BaseCumulativeTransformOutputParser` (re-parse the whole buffer,
/// emit the delta) but with the *hold-back* discipline its XML parser
/// uses, because our goal is to suppress an envelope rather than
/// surface a structure and our transport cannot retract already-sent
/// bytes.
#[derive(Debug, Default)]
pub struct ToolCallStreamFilter {
    /// Bytes of *stripped* output already forwarded to the client.
    emitted_len: usize,
}

impl ToolCallStreamFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the cumulative round content seen so far; returns the text
    /// now safe to forward (may be empty when everything new is still
    /// inside, or potentially inside, an envelope).
    pub fn push(&mut self, cumulative_content: &str) -> String {
        let stripped = strip_tool_call_envelopes(cumulative_content);
        let safe_end = safe_emit_boundary(&stripped);
        self.emit_up_to(&stripped, safe_end)
    }

    /// Flush at end of stream: everything remaining in the stripped
    /// buffer is final. A dangling unclosed envelope is emitted verbatim
    /// here, exactly as [`strip_tool_call_envelopes`] leaves it in the
    /// persisted message, keeping the live and stored views consistent.
    pub fn finish(&mut self, cumulative_content: &str) -> String {
        let stripped = strip_tool_call_envelopes(cumulative_content);
        let end = stripped.len();
        self.emit_up_to(&stripped, end)
    }

    fn emit_up_to(&mut self, stripped: &str, end: usize) -> String {
        // Defensive guards: `end` must be a valid, forward, in-bounds
        // char boundary. Given the emitted prefix is stable across
        // pushes (we never emit past an unresolved envelope), these
        // never trip in practice — but a malformed boundary must yield
        // nothing rather than panic.
        if end <= self.emitted_len
            || end > stripped.len()
            || !stripped.is_char_boundary(self.emitted_len)
            || !stripped.is_char_boundary(end)
        {
            return String::new();
        }
        let out = stripped[self.emitted_len..end].to_string();
        self.emitted_len = end;
        out
    }
}

/// Largest byte offset of `stripped` that is safe to emit: everything
/// before the first dangling `<tool_call` opener (complete envelopes
/// are already gone, so any opener left is unclosed), or before a
/// trailing partial `<tool_call` prefix — whichever comes first.
fn safe_emit_boundary(stripped: &str) -> usize {
    if let Some(m) = TOOL_CALL_OPEN_RE.find(stripped) {
        return m.start();
    }
    let hold = trailing_marker_prefix_len(stripped, TOOL_CALL_OPEN_MARKER);
    stripped.len() - hold
}

/// Length of the longest suffix of `buf` that is a proper, non-empty
/// prefix of `marker` (so a `<tool_c` at the tail of a chunk is held
/// back until the next chunk resolves it). Returns 0 when no suffix
/// matches. UTF-8 safe — only compares at char boundaries.
fn trailing_marker_prefix_len(buf: &str, marker: &str) -> usize {
    let max_k = buf.len().min(marker.len().saturating_sub(1));
    for k in (1..=max_k).rev() {
        let start = buf.len() - k;
        if buf.is_char_boundary(start) && buf.as_bytes()[start..] == marker.as_bytes()[..k] {
            return k;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- parser ------------------------------------------------------------

    #[test]
    fn extracts_zero_arg_call() {
        let calls = extract_tool_calls("<tool_call>twitter_get_page_context</tool_call>");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "twitter_get_page_context");
        assert!(calls[0].args.as_object().unwrap().is_empty());
    }

    #[test]
    fn extracts_call_with_typed_args() {
        let calls = extract_tool_calls(
            "<tool_call>search<arg_key>query</arg_key><arg_value>rust</arg_value>\
<arg_key>limit</arg_key><arg_value>5</arg_value></tool_call>",
        );
        assert_eq!(calls.len(), 1);
        let args = calls[0].args.as_object().unwrap();
        assert_eq!(args.get("query"), Some(&json!("rust")));
        assert_eq!(args.get("limit"), Some(&json!(5)));
    }

    #[test]
    fn strip_removes_envelope_keeps_prose() {
        let stripped = strip_tool_call_envelopes(
            "Before.<tool_call>foo<arg_key>k</arg_key><arg_value>v</arg_value></tool_call>After.",
        );
        assert_eq!(stripped, "Before.After.");
    }

    // ---- safe_emit_boundary ------------------------------------------------

    #[test]
    fn boundary_holds_at_dangling_open() {
        // Complete envelopes are already gone before this runs; a lone
        // opener means its close hasn't arrived → hold from it.
        assert_eq!(safe_emit_boundary("hello <tool_call>foo"), 6);
    }

    #[test]
    fn boundary_holds_trailing_partial_marker() {
        // `<tool_c` at the tail could become `<tool_call>`.
        assert_eq!(safe_emit_boundary("hello <tool_c"), 6);
        // A lone `<` is also a partial marker prefix.
        assert_eq!(safe_emit_boundary("hello <"), 6);
    }

    #[test]
    fn boundary_does_not_hold_plural_tool_calls() {
        // `<tool_calls>` is not an opener (`\b` mismatch) and has no
        // trailing partial, so everything is safe to emit.
        let s = "see <tool_calls> elsewhere";
        assert_eq!(safe_emit_boundary(s), s.len());
    }

    #[test]
    fn boundary_does_not_hold_unrelated_less_than() {
        let s = "if x < 10 then";
        assert_eq!(safe_emit_boundary(s), s.len());
    }

    // ---- ToolCallStreamFilter ----------------------------------------------

    /// Drive the filter with a sequence of cumulative snapshots and
    /// collect everything it forwarded, including the final flush.
    fn run_filter(snapshots: &[&str]) -> String {
        let mut filter = ToolCallStreamFilter::new();
        let mut out = String::new();
        for snap in snapshots {
            out.push_str(&filter.push(snap));
        }
        out.push_str(&filter.finish(snapshots.last().copied().unwrap_or("")));
        out
    }

    #[test]
    fn suppresses_envelope_split_across_chunks() {
        // The opener arrives split: `<tool` then `_call>…`.
        let out = run_filter(&[
            "Let me look.",
            "Let me look.<tool",
            "Let me look.<tool_call>twitter_get_page_context",
            "Let me look.<tool_call>twitter_get_page_context</tool_call>",
        ]);
        assert_eq!(out, "Let me look.");
        assert!(!out.contains("<tool_call"));
        assert!(!out.contains("twitter_get_page_context"));
    }

    #[test]
    fn suppresses_whole_envelope_in_one_chunk() {
        let out = run_filter(&[
            "Answer.<tool_call>foo<arg_key>k</arg_key><arg_value>v</arg_value></tool_call>",
        ]);
        assert_eq!(out, "Answer.");
    }

    #[test]
    fn emits_prose_before_and_after_envelope() {
        let out = run_filter(&[
            "Before. ",
            "Before. <tool_call>foo</tool_call>",
            "Before. <tool_call>foo</tool_call> After.",
        ]);
        assert_eq!(out, "Before.  After.");
    }

    #[test]
    fn suppresses_multiple_envelopes() {
        let out = run_filter(&[
            "<tool_call>a</tool_call>",
            "<tool_call>a</tool_call>mid",
            "<tool_call>a</tool_call>mid<tool_call>b</tool_call>",
            "<tool_call>a</tool_call>mid<tool_call>b</tool_call>end",
        ]);
        assert_eq!(out, "midend");
    }

    #[test]
    fn lone_less_than_is_emitted_not_held_forever() {
        // A `<` that never becomes a tool tag must surface — held one
        // snapshot, then released once disambiguated.
        let out = run_filter(&["x < ", "x < 10 done"]);
        assert_eq!(out, "x < 10 done");
    }

    #[test]
    fn finish_emits_dangling_unclosed_envelope_for_consistency() {
        // The model opened a call and stopped without closing it. The
        // stripper leaves it in the persisted content, so the stream
        // must surface it too — otherwise live and reload disagree.
        let out = run_filter(&["Hi.<tool_call>foo"]);
        assert_eq!(out, "Hi.<tool_call>foo");
    }

    #[test]
    fn finish_emits_trailing_partial_marker_as_text() {
        // A `<too` at the very end of the stream never became a tool
        // tag → it is real text and must be flushed at finish.
        let out = run_filter(&["price <too"]);
        assert_eq!(out, "price <too");
    }

    #[test]
    fn empty_pushes_emit_nothing() {
        let mut filter = ToolCallStreamFilter::new();
        assert_eq!(filter.push(""), "");
        assert_eq!(filter.push(""), "");
        assert_eq!(filter.finish(""), "");
    }

    #[test]
    fn forwarded_stream_equals_stripped_content() {
        // The core invariant: whatever the filter forwards across the
        // whole stream must equal `strip_tool_call_envelopes` of the
        // final content — i.e. live view == persisted view.
        let final_content = "Intro <tool_call>a<arg_key>k</arg_key>\
<arg_value>v</arg_value></tool_call> middle <tool_call>b</tool_call> end";
        let snapshots: Vec<String> = {
            // Simulate streaming by growing the content one word at a time.
            let mut acc = String::new();
            let mut snaps = Vec::new();
            for word in final_content.split_inclusive(' ') {
                acc.push_str(word);
                snaps.push(acc.clone());
            }
            snaps
        };
        let refs: Vec<&str> = snapshots.iter().map(String::as_str).collect();
        let forwarded = run_filter(&refs);
        assert_eq!(forwarded, strip_tool_call_envelopes(final_content));
    }
}
