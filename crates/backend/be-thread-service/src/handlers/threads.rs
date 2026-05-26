use std::sync::Arc;

use agent_chain::messages::AnyMessage;
use agent_chain::{HumanMessage, SystemMessage};
use agent_chain_core::messages::ContentBlocks;
use axum::Json;
use axum::extract::{Path, Query, State};
use be_auth_core::AuthUser;
use be_remote_db::{Message, MessageType, PaginationParams};
use thread_core::{
    CreateThreadRequest, CreateThreadResponse, DeleteThreadResponse, GenerateThreadTitleRequest,
    GenerateThreadTitleResponse, GetThreadResponse, ListThreadsQuery, ListThreadsResponse,
};
use uuid::Uuid;

use crate::conversion::db_thread_to_wire;
use crate::error::ThreadServiceResult;
use crate::service::AppState;

const TITLE_DEFAULT: &str = "New Chat";
/// Number of recent messages to feed the title model. Reference projects
/// converge on 1-2 (Vercel uses just the first user turn; Open WebUI uses
/// the last two). More than that biases the title toward whatever the
/// assistant said most recently rather than what the user asked.
const TITLE_CONTEXT_MESSAGE_LIMIT: u32 = 2;
const TITLE_MAX_WORDS: usize = 6;
/// Per-turn character cap for the flattened transcript. Titles never need
/// more than this much context, and capping keeps long pastes or large
/// asset references from blowing up the title-model prompt.
const TITLE_TURN_CHAR_LIMIT: usize = 500;
const LIST_DEFAULT_LIMIT: u32 = 20;
const LIST_DEFAULT_OFFSET: u32 = 0;

const TITLE_SYSTEM_PROMPT: &str = "You generate short titles for chat conversations.

Rules:
- Output ONLY the title text. Nothing before it, nothing after it.
- 2-6 words. Sentence case.
- No markdown: no **, no _, no #, no backticks, no code fences.
- No quotation marks. No \"Title:\" prefix. No trailing punctuation.
- Summarize the user's topic, not the assistant's response. Never echo \
  refusals like \"I can't help with that\" — describe what the user was \
  trying to do.
- If the topic is unclear, output: New conversation

Examples:
- User asks how to deploy a Rust service to Fly.io  ->  Deploy Rust service to Fly.io
- User asks to search the web for React 19 features  ->  Search for React 19 features
- User pastes a stack trace and asks for help  ->  Debugging a stack trace
- User says \"hi\"  ->  New conversation";

#[tracing::instrument(skip(state, user, body))]
pub async fn create_thread(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<CreateThreadRequest>,
) -> ThreadServiceResult<Json<CreateThreadResponse>> {
    let user_id = user.user_id()?;
    let title = body
        .title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| TITLE_DEFAULT.to_string());

    let thread = state
        .db
        .create_thread()
        .user_id(user_id)
        .title(title)
        .call()
        .await?;

    tracing::info!("Created thread {}", thread.id);

    Ok(Json(CreateThreadResponse {
        thread: db_thread_to_wire(thread),
    }))
}

#[tracing::instrument(skip(state, user))]
pub async fn list_threads(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query): Query<ListThreadsQuery>,
) -> ThreadServiceResult<Json<ListThreadsResponse>> {
    let user_id = user.user_id()?;
    let limit = query.limit.unwrap_or(LIST_DEFAULT_LIMIT);
    let offset = query.offset.unwrap_or(LIST_DEFAULT_OFFSET);

    let threads = state
        .db
        .list_threads()
        .user_id(user_id)
        .params(PaginationParams::new(offset, limit, "DESC"))
        .call()
        .await?;

    Ok(Json(ListThreadsResponse {
        threads: threads.into_iter().map(db_thread_to_wire).collect(),
    }))
}

/// List threads linked (via `activity_threads`) to a single activity.
///
/// Powers the desktop sidebar's per-app filter: as the user cycles through
/// the timeline rail, the frontend fetches the threads associated with the
/// currently-active activity and shows them as the filtered list.
#[tracing::instrument(skip(state, user), fields(activity_id = %activity_id))]
pub async fn list_threads_for_activity(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(activity_id): Path<Uuid>,
    Query(query): Query<ListThreadsQuery>,
) -> ThreadServiceResult<Json<ListThreadsResponse>> {
    let user_id = user.user_id()?;
    let limit = query.limit.unwrap_or(LIST_DEFAULT_LIMIT);
    let offset = query.offset.unwrap_or(LIST_DEFAULT_OFFSET);

    let threads = state
        .db
        .list_threads_for_activity()
        .user_id(user_id)
        .activity_id(activity_id)
        .params(PaginationParams::new(offset, limit, "DESC"))
        .call()
        .await?;

    Ok(Json(ListThreadsResponse {
        threads: threads.into_iter().map(db_thread_to_wire).collect(),
    }))
}

#[tracing::instrument(skip(state, user), fields(thread_id = %thread_id))]
pub async fn get_thread(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
) -> ThreadServiceResult<Json<GetThreadResponse>> {
    let user_id = user.user_id()?;

    let thread = state
        .db
        .get_thread()
        .id(thread_id)
        .user_id(user_id)
        .call()
        .await?;

    Ok(Json(GetThreadResponse {
        thread: db_thread_to_wire(thread),
    }))
}

#[tracing::instrument(skip(state, user), fields(thread_id = %thread_id))]
pub async fn delete_thread(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
) -> ThreadServiceResult<Json<DeleteThreadResponse>> {
    let user_id = user.user_id()?;

    state
        .db
        .delete_thread()
        .id(thread_id)
        .user_id(user_id)
        .call()
        .await?;

    tracing::info!("Deleted thread {}", thread_id);

    Ok(Json(DeleteThreadResponse {}))
}

/// Token-gated. The `be-authz` middleware checks the user's monthly token
/// limit before this handler runs; on exhaustion it short-circuits with a
/// 429 and this code never executes.
///
/// Idempotency: if the thread already has a user-meaningful title (anything
/// other than the freshly-created default), this returns the existing
/// thread unchanged. The auto-title is only meant to replace the placeholder
/// — once a real title exists (auto-generated or user-renamed), we don't
/// overwrite.
#[tracing::instrument(skip(state, user), fields(thread_id = %thread_id))]
pub async fn generate_thread_title(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
    Json(_): Json<GenerateThreadTitleRequest>,
) -> ThreadServiceResult<Json<GenerateThreadTitleResponse>> {
    let user_id = user.user_id()?;

    let existing = state
        .db
        .get_thread()
        .id(thread_id)
        .user_id(user_id)
        .call()
        .await?;
    if let Some(current) = existing.title.as_deref()
        && !current.trim().is_empty()
        && current != TITLE_DEFAULT
    {
        return Ok(Json(GenerateThreadTitleResponse {
            thread: db_thread_to_wire(existing),
        }));
    }

    let recent_messages = state
        .db
        .list_messages()
        .thread_id(thread_id)
        .user_id(user_id)
        .params(PaginationParams::new(
            0,
            TITLE_CONTEXT_MESSAGE_LIMIT,
            "DESC",
        ))
        .call()
        .await?;

    let title = match build_transcript(recent_messages) {
        Some(transcript) => {
            let messages = build_title_prompt(&transcript);
            let title_provider = state.providers.title.clone();
            match title_provider.invoke(messages, None).await {
                Ok(message) => sanitize_title(&message.content.to_string())
                    .unwrap_or_else(|| TITLE_DEFAULT.to_string()),
                Err(e) => {
                    tracing::warn!("Title model failed, falling back to default: {e}");
                    TITLE_DEFAULT.to_string()
                }
            }
        }
        None => {
            tracing::debug!("No usable text content in recent messages; using default title");
            TITLE_DEFAULT.to_string()
        }
    };

    let thread = state
        .db
        .update_thread()
        .id(thread_id)
        .user_id(user_id)
        .title(title)
        .call()
        .await?;

    Ok(Json(GenerateThreadTitleResponse {
        thread: db_thread_to_wire(thread),
    }))
}

/// Flatten the recent message rows into a `User: ... / Assistant: ...`
/// transcript suitable for embedding inside a single user-role prompt.
///
/// The DB returns rows newest-first (DESC) because that's what every other
/// caller of `list_messages` wants for pagination — we reverse here so the
/// transcript reads in chronological order.
///
/// We drop everything that would confuse the title model:
///
/// - `Tool` and `System` rows entirely (tool results dominate token count
///   and bias the title toward tool-result phrasing; system rows are
///   internal and never topical).
/// - Non-text content blocks (tool calls, reasoning, images, files, etc.).
///   Asset references stay as references — we never inline their bytes —
///   but their content blocks carry no human-readable summary, so they
///   contribute nothing to a topic label.
///
/// Returns `None` if no human-readable text remains after filtering — the
/// caller short-circuits to the default title rather than asking the model
/// to summarise an empty transcript.
fn build_transcript(rows: Vec<Message>) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut saw_user_text = false;

    for row in rows.into_iter().rev() {
        let role = match row.message_type {
            MessageType::Human => "User",
            MessageType::Ai => "Assistant",
            MessageType::Tool | MessageType::System => continue,
        };

        let Some(text) = text_from_content(row.content) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        if role == "User" {
            saw_user_text = true;
        }

        let clamped = clamp_chars(&text, TITLE_TURN_CHAR_LIMIT);
        lines.push(format!("{role}: {clamped}"));
    }

    if !saw_user_text {
        return None;
    }
    Some(lines.join("\n"))
}

/// Extract the joined text from a stored `content` JSON blob, keeping only
/// `text` blocks and collapsing whitespace. Other block kinds (tool calls,
/// reasoning, images, file references, etc.) are skipped — `ContentBlocks`'
/// `Display` impl already filters to text-only — so a message that consists
/// solely of, say, an image reference returns an empty string here.
fn text_from_content(content: serde_json::Value) -> Option<String> {
    let blocks: ContentBlocks = serde_json::from_value(content).ok()?;
    let joined = blocks.to_string();
    let collapsed: String = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    Some(collapsed)
}

/// Truncate `s` to at most `max` chars (not bytes), appending `…` if the
/// string was cut. Operates on `char` boundaries to keep multi-byte
/// codepoints intact.
fn clamp_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Build the two-message prompt the title model sees:
///
/// - a system message with the title-generation rules and few-shot examples
/// - a single user message containing the flattened transcript inside a
///   `<conversation>` block, terminated by a `Title:` anchor
///
/// The anchor is the key reason this prompt shape works reliably: the
/// model's natural continuation of `Title:` is the title itself, not a
/// continuation of the assistant turn it just read.
fn build_title_prompt(transcript: &str) -> Vec<AnyMessage> {
    let user_content = format!(
        "Summarize the following conversation as a title.\n\n\
         <conversation>\n{transcript}\n</conversation>\n\n\
         Title:"
    );
    vec![
        SystemMessage::builder()
            .content(TITLE_SYSTEM_PROMPT.to_string())
            .build()
            .into(),
        HumanMessage::builder().content(user_content).build().into(),
    ]
}

/// Clean up the raw model output into a presentable title, or `None` if
/// nothing usable remains.
///
/// In order:
///
/// 1. Drop `<think>...</think>` reasoning blocks that some models leak
///    even when not asked for reasoning.
/// 2. Drop a leading `Title:` / `Title -` label.
/// 3. Strip wrapping markdown / quote / backtick characters, repeating
///    until the string is stable so e.g. `**"Foo"**` collapses to `Foo`.
/// 4. Drop trailing sentence punctuation.
/// 5. Collapse internal whitespace.
/// 6. Clamp to [`TITLE_MAX_WORDS`] words.
/// 7. Capitalise the first character.
fn sanitize_title(raw: &str) -> Option<String> {
    let without_think = strip_think_blocks(raw);
    let without_label = strip_title_label(without_think.trim());
    let unwrapped = strip_wrapping_markers(without_label);
    let untrailing = unwrapped.trim_end_matches(['.', '!', '?', ',', ';', ':']);
    let collapsed: String = untrailing.split_whitespace().collect::<Vec<_>>().join(" ");
    let clamped = collapsed
        .split_whitespace()
        .take(TITLE_MAX_WORDS)
        .collect::<Vec<_>>()
        .join(" ");

    if clamped.is_empty() {
        return None;
    }
    Some(capitalize_first(&clamped))
}

/// Remove `<think>...</think>` blocks (case-insensitive, multi-line). Some
/// reasoning-tuned models emit these even when the prompt doesn't ask for
/// thinking. We don't use `regex` here — a hand-rolled scan over `<think`
/// and `</think>` keeps the dependency surface small and is faster than
/// compiling a regex on every call.
fn strip_think_blocks(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut cursor = 0;
    while cursor < s.len() {
        let Some(rel_open) = lower[cursor..].find("<think") else {
            out.push_str(&s[cursor..]);
            break;
        };
        let open = cursor + rel_open;
        out.push_str(&s[cursor..open]);
        // Skip past the opening tag up to its closing `>`.
        let after_tag = match s[open..].find('>') {
            Some(rel) => open + rel + 1,
            None => break,
        };
        // Find the matching close tag; if absent, drop the rest.
        let Some(rel_close) = lower[after_tag..].find("</think>") else {
            break;
        };
        cursor = after_tag + rel_close + "</think>".len();
    }
    out
}

/// Drop a leading `Title:` / `Title -` (case-insensitive) prefix, including
/// any whitespace between the label and the title text.
fn strip_title_label(s: &str) -> &str {
    let trimmed = s.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("title") {
        return trimmed;
    }
    let after_word = &trimmed["title".len()..];
    let after_sep = after_word.trim_start();
    let Some(first) = after_sep.chars().next() else {
        return trimmed;
    };
    if !matches!(first, ':' | '-' | '—' | '–') {
        return trimmed;
    }
    after_sep[first.len_utf8()..].trim_start()
}

/// Strip wrapping markdown / quote / backtick characters from both ends,
/// repeating until the string stabilises. Catches cases like
/// `**"Foo"**` or `"**Foo**"` where one strip pass isn't enough.
fn strip_wrapping_markers(s: &str) -> String {
    const MARKERS: &[char] = &['*', '_', '#', '`', '"', '\'', '“', '”', '‘', '’', ' ', '\t'];
    let mut current = s.to_string();
    loop {
        let next = current.trim_matches(MARKERS).to_string();
        if next == current {
            return next;
        }
        current = next;
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use be_remote_db::{Message, MessageType};
    use chrono::Utc;
    use serde_json::json;

    fn make_message(message_type: MessageType, content: serde_json::Value) -> Message {
        Message {
            id: Uuid::now_v7(),
            thread_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            parent_message_id: None,
            message_type,
            content,
            tool_call_id: None,
            tool_calls: None,
            additional_kwargs: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn text_blocks(text: &str) -> serde_json::Value {
        json!([{ "type": "text", "text": text }])
    }

    #[test]
    fn capitalize_first_handles_empty_and_unicode() {
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("hello world"), "Hello world");
        assert_eq!(capitalize_first("über"), "Über");
    }

    #[test]
    fn sanitize_title_strips_bold_markdown() {
        assert_eq!(
            sanitize_title("**Hello world**").as_deref(),
            Some("Hello world")
        );
    }

    #[test]
    fn sanitize_title_strips_heading_marks() {
        assert_eq!(
            sanitize_title("# My great title").as_deref(),
            Some("My great title")
        );
    }

    #[test]
    fn sanitize_title_strips_quotes_and_smart_quotes() {
        assert_eq!(
            sanitize_title("\"Quoted title\"").as_deref(),
            Some("Quoted title")
        );
        assert_eq!(
            sanitize_title("“Smart-quoted title”").as_deref(),
            Some("Smart-quoted title")
        );
    }

    #[test]
    fn sanitize_title_strips_title_prefix() {
        assert_eq!(
            sanitize_title("Title: Something here").as_deref(),
            Some("Something here")
        );
        assert_eq!(
            sanitize_title("title - Other thing").as_deref(),
            Some("Other thing")
        );
    }

    #[test]
    fn sanitize_title_strips_code_fences_and_backticks() {
        assert_eq!(sanitize_title("`title`").as_deref(), Some("Title"));
        assert_eq!(
            sanitize_title("```Code title```").as_deref(),
            Some("Code title")
        );
    }

    #[test]
    fn sanitize_title_strips_think_blocks() {
        assert_eq!(
            sanitize_title("<think>let me think about this</think>Final title").as_deref(),
            Some("Final title")
        );
        assert_eq!(
            sanitize_title("<think>x</think>  **Wrapped**").as_deref(),
            Some("Wrapped")
        );
    }

    #[test]
    fn sanitize_title_handles_layered_wrappers() {
        assert_eq!(
            sanitize_title("**\"Foo bar\"**").as_deref(),
            Some("Foo bar")
        );
        assert_eq!(
            sanitize_title("\"**Foo bar**\"").as_deref(),
            Some("Foo bar")
        );
    }

    #[test]
    fn sanitize_title_strips_trailing_punctuation() {
        assert_eq!(
            sanitize_title("A great title.").as_deref(),
            Some("A great title")
        );
        assert_eq!(
            sanitize_title("Something happened!").as_deref(),
            Some("Something happened")
        );
    }

    #[test]
    fn sanitize_title_clamps_word_count() {
        assert_eq!(
            sanitize_title("one two three four five six seven eight").as_deref(),
            Some("One two three four five six")
        );
    }

    #[test]
    fn sanitize_title_collapses_internal_whitespace() {
        assert_eq!(
            sanitize_title("  hello   \t world  ").as_deref(),
            Some("Hello world")
        );
    }

    #[test]
    fn sanitize_title_returns_none_for_empty() {
        assert_eq!(sanitize_title(""), None);
        assert_eq!(sanitize_title("   "), None);
        assert_eq!(sanitize_title("**  **"), None);
        assert_eq!(sanitize_title("<think>only thinking</think>"), None);
    }

    #[test]
    fn strip_think_blocks_handles_no_close_tag() {
        // Unterminated <think> drops everything from the open onward.
        assert_eq!(strip_think_blocks("before <think>oops"), "before ");
    }

    #[test]
    fn strip_think_blocks_handles_multiple_blocks() {
        assert_eq!(
            strip_think_blocks("a<think>x</think>b<think>y</think>c"),
            "abc"
        );
    }

    #[test]
    fn strip_title_label_only_strips_recognised_separators() {
        assert_eq!(strip_title_label("Title: Foo"), "Foo");
        assert_eq!(strip_title_label("title — Foo"), "Foo");
        // No separator after "title" — leave it alone.
        assert_eq!(
            strip_title_label("Titles of nobility"),
            "Titles of nobility"
        );
    }

    #[test]
    fn clamp_chars_preserves_codepoint_boundaries() {
        assert_eq!(clamp_chars("héllo", 10), "héllo");
        let clamped = clamp_chars("ünicode-test-string", 5);
        // 4 chars + ellipsis = 5 chars visible
        assert_eq!(clamped.chars().count(), 5);
        assert!(clamped.ends_with('…'));
    }

    #[test]
    fn build_transcript_drops_tool_and_system_rows() {
        let rows = vec![
            // Newest-first as the DB returns them.
            make_message(MessageType::Tool, text_blocks("huge json result")),
            make_message(MessageType::Ai, text_blocks("Searching...")),
            make_message(MessageType::System, text_blocks("internal")),
            make_message(MessageType::Human, text_blocks("search the web for X")),
        ];
        let transcript = build_transcript(rows).expect("transcript should exist");
        // Chronological order: Human first, then Ai. System + Tool dropped.
        assert_eq!(
            transcript,
            "User: search the web for X\nAssistant: Searching..."
        );
    }

    #[test]
    fn build_transcript_returns_none_without_user_text() {
        let rows = vec![
            make_message(MessageType::Ai, text_blocks("hello")),
            make_message(MessageType::Human, json!([])),
        ];
        assert!(build_transcript(rows).is_none());
    }

    #[test]
    fn build_transcript_skips_messages_with_only_non_text_blocks() {
        // A human message that's purely an image reference contributes no
        // topical text — but if it's the *only* user input, we'd rather
        // fall back to the default title than ask the model to summarise
        // a blank transcript.
        let rows = vec![make_message(
            MessageType::Human,
            json!([{
                "type": "image",
                "url": "https://example.com/img.png"
            }]),
        )];
        assert!(build_transcript(rows).is_none());
    }

    #[test]
    fn build_transcript_clamps_long_turns() {
        let huge = "x".repeat(TITLE_TURN_CHAR_LIMIT + 200);
        let rows = vec![make_message(MessageType::Human, text_blocks(&huge))];
        let transcript = build_transcript(rows).expect("transcript should exist");
        // "User: " prefix + clamped content (max chars + …)
        let user_part = transcript.strip_prefix("User: ").unwrap();
        assert_eq!(user_part.chars().count(), TITLE_TURN_CHAR_LIMIT);
        assert!(user_part.ends_with('…'));
    }

    #[test]
    fn build_transcript_collapses_whitespace_in_text_blocks() {
        let rows = vec![make_message(
            MessageType::Human,
            text_blocks("hello\n\n\tworld   here"),
        )];
        let transcript = build_transcript(rows).unwrap();
        assert_eq!(transcript, "User: hello world here");
    }
}
