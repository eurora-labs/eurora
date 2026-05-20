//! Per-turn tool catalog and active-context system message.
//!
//! The catalog merges the backend's in-process tools (Firecrawl,
//! `describe_image`) with the wire-side [`WireToolDescriptor`]s the client
//! advertises in [`thread_core::ChatClientMessage::CapabilityUpdate`]. The
//! agent loop dispatches against this catalog, picking the right execution
//! path per call. Server-local tools run in-process; remote tools transit
//! through [`crate::remote_tool_bus::ChatRemoteBus`] back to the client.
//!
//! The catalog is constructed per turn so it always reflects the client's
//! freshest state — context activations and deactivations between turns
//! change the visible tool set without needing a long-lived reconcile loop.
//!
//! [`build_context_system_message`] renders the active contexts into a
//! single LLM-visible `SystemMessage`. The chat handler prepends it to the
//! message list when assembling the per-turn LLM context.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};

use agent_chain::{BaseTool, SystemMessage, language_models::ToolLike};
use serde_json::Value;
use thiserror::Error;
use thread_core::{WireActiveContext, WireToolDescriptor};

/// A single entry in a [`TurnCatalog`]. The variant determines the dispatch
/// path taken by the agent loop.
#[derive(Clone)]
pub enum TurnEntry {
    /// Tool runs in-process on the backend (Firecrawl, `describe_image`).
    ServerLocal { tool: Arc<dyn BaseTool> },
    /// Tool runs on the client; the agent loop hands the descriptor + args
    /// to [`crate::remote_tool_bus::ChatRemoteBus`] and awaits the result.
    Remote { descriptor: WireToolDescriptor },
}

/// Errors that can surface while assembling a [`TurnCatalog`].
///
/// The single variant covers descriptor-name collisions, which are a
/// protocol-level fault: two tools claim the same fully-qualified name and
/// the dispatcher would be ambiguous. The chat handler renders this as
/// `ChatServerMessage::Error { kind: "protocol", ... }`.
#[derive(Debug, Error)]
pub enum CatalogBuildError {
    #[error("tool name collision: {0}")]
    NameCollision(String),
}

/// One source of dispatch truth for an entire turn.
///
/// Built once at turn start from the LLM context's server-local tools plus
/// whatever the client advertised in [`thread_core::ChatClientMessage::CapabilityUpdate`].
/// The catalog is then held in an `Arc` and shared between
/// [`crate::agent_loop::run_agent_loop`] and the LLM-binding code that
/// constructs the `ToolLike` list.
///
/// [`Self::tool_likes`] is memoized via [`OnceLock`]: the first caller
/// builds the `Arc<[ToolLike]>`; subsequent callers (initial bind in
/// `llm::context`, forced-synthesis rebind in `agent_loop`) cheaply
/// clone the `Arc`.
#[derive(Default)]
pub struct TurnCatalog {
    entries: HashMap<String, TurnEntry>,
    tool_likes_cache: OnceLock<Arc<[ToolLike]>>,
}

impl std::fmt::Debug for TurnCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kinds: HashMap<&str, &'static str> = self
            .entries
            .iter()
            .map(|(name, entry)| {
                let kind = match entry {
                    TurnEntry::ServerLocal { .. } => "server_local",
                    TurnEntry::Remote { .. } => "remote",
                };
                (name.as_str(), kind)
            })
            .collect();
        f.debug_struct("TurnCatalog")
            .field("entries", &kinds)
            .finish()
    }
}

impl TurnCatalog {
    /// Build the catalog by merging server-local tools with the remote
    /// descriptors from a `CapabilityUpdate`.
    ///
    /// Remote descriptors whose `required_contexts` aren't all satisfied by
    /// `active_contexts` are silently dropped: the LLM should not be told
    /// about a tool that can't dispatch this turn. The client filters too,
    /// but the server is the authority for what gets bound.
    ///
    /// Name collisions — between server-local entries, between remote
    /// entries, or across the two sets — fail the build.
    pub fn build(
        server_local: impl IntoIterator<Item = Arc<dyn BaseTool>>,
        remote: impl IntoIterator<Item = WireToolDescriptor>,
        active_contexts: &[WireActiveContext],
    ) -> Result<Self, CatalogBuildError> {
        let active_keys: std::collections::HashSet<&str> =
            active_contexts.iter().map(|c| c.key.as_str()).collect();

        let mut entries: HashMap<String, TurnEntry> = HashMap::new();

        for tool in server_local {
            let name = tool.name().to_string();
            if entries.contains_key(&name) {
                return Err(CatalogBuildError::NameCollision(name));
            }
            entries.insert(name, TurnEntry::ServerLocal { tool });
        }

        for descriptor in remote {
            let contexts_satisfied = descriptor
                .required_contexts
                .iter()
                .all(|c| active_keys.contains(c.as_str()));
            if !contexts_satisfied {
                continue;
            }
            let name = descriptor.name().to_owned();
            if entries.contains_key(&name) {
                return Err(CatalogBuildError::NameCollision(name));
            }
            entries.insert(name, TurnEntry::Remote { descriptor });
        }

        Ok(Self {
            entries,
            tool_likes_cache: OnceLock::new(),
        })
    }

    /// Look up an entry by fully-qualified tool name. Returns `None` for
    /// names the LLM hallucinated.
    pub fn get(&self, name: &str) -> Option<&TurnEntry> {
        self.entries.get(name)
    }

    /// LLM-bind shape. Server-local entries become [`ToolLike::Tool`] so the
    /// chat model can invoke them through `BaseTool`; remote entries become
    /// [`ToolLike::Definition`] so the LLM sees them in the function-tool
    /// list without being able to invoke them locally.
    ///
    /// The first call materializes the `Vec` and stores it as an
    /// `Arc<[ToolLike]>` in [`Self::tool_likes_cache`]; subsequent calls
    /// (initial LLM bind in `llm::context` + forced-synthesis rebind in
    /// `agent_loop`) cheaply clone the `Arc`.
    pub fn tool_likes(&self) -> Arc<[ToolLike]> {
        Arc::clone(self.tool_likes_cache.get_or_init(|| {
            self.entries
                .values()
                .map(|entry| match entry {
                    TurnEntry::ServerLocal { tool } => ToolLike::Tool(tool.clone()),
                    TurnEntry::Remote { descriptor } => {
                        ToolLike::Definition(descriptor.definition.clone())
                    }
                })
                .collect::<Vec<_>>()
                .into()
        }))
    }

    /// Number of entries — useful for the "skip LLM tool binding when
    /// there's nothing to bind" guard.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Render the live contexts the client advertised into a single LLM-facing
/// system message. Returns `None` when there are no contexts — the chat
/// handler skips the prepend in that case.
///
/// Per-key formatters live below; new context kinds add an arm to the
/// `match` and a private `format_<key>` helper.
pub fn build_context_system_message(contexts: &[WireActiveContext]) -> Option<SystemMessage> {
    if contexts.is_empty() {
        return None;
    }

    let mut buf = String::from("Live context from the user's machine:\n\n");
    for ctx in contexts {
        match ctx.key.as_str() {
            "youtube::watch_page" => format_youtube_watch_page(&mut buf, &ctx.data),
            _ => format_generic(&mut buf, ctx),
        }
    }
    buf.push_str(
        "\nTool calls for these contexts are pinned to the specific source the user was on at \
         turn start. If the user closes the tab or navigates away, calls return \
         `context_unavailable` — acknowledge the change to the user rather than retrying.",
    );

    Some(SystemMessage::builder().content(buf).build())
}

fn format_youtube_watch_page(buf: &mut String, data: &Value) {
    let title = data
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("(unknown)");
    let channel = data
        .get("channel")
        .and_then(Value::as_str)
        .unwrap_or("(unknown)");
    let duration = data
        .get("duration_seconds")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let approx = data
        .get("approximate_timestamp_seconds")
        .and_then(Value::as_f64);

    writeln!(buf, "## YouTube video (currently playing)").expect("write to String");
    writeln!(buf, "- Title: {title}").expect("write to String");
    writeln!(buf, "- Channel: {channel}").expect("write to String");
    writeln!(buf, "- Duration: {}", fmt_hms(duration)).expect("write to String");
    if let Some(t) = approx {
        writeln!(
            buf,
            "- Approximate current playback time: {} (call `browser::youtube::get_current_timestamp` for precise)",
            fmt_hms(t)
        )
        .expect("write to String");
    }
}

fn format_generic(buf: &mut String, ctx: &WireActiveContext) {
    writeln!(buf, "## Context `{}`", ctx.key).expect("write to String");
    let pretty = serde_json::to_string_pretty(&ctx.data).unwrap_or_else(|_| ctx.data.to_string());
    writeln!(buf, "```json\n{pretty}\n```").expect("write to String");
}

/// Format a non-negative number of seconds as `H:MM:SS` (or `M:SS` if under
/// an hour). Negative or non-finite inputs round to zero.
fn fmt_hms(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".to_string();
    }
    let total = seconds.round() as u64;
    let hours = total / 3_600;
    let minutes = (total % 3_600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use agent_chain::async_trait;
    use agent_chain::callbacks::manager::CallbackManagerForToolRun;
    use agent_chain::error::Result as ToolResult;
    use agent_chain::runnables::RunnableConfig;
    use agent_chain::tools::{ArgsSchema, BaseTool, ToolInput, ToolOutput};
    use chrono::{DateTime, Utc};
    use serde_json::json;
    use thread_core::{WireActiveContext, WireToolDescriptor};

    /// Minimal `BaseTool` with a configurable name — used to construct
    /// collision and dispatch fixtures without dragging the full Firecrawl
    /// surface into tests.
    #[derive(Debug)]
    struct NamedTool {
        name: String,
        args_schema: ArgsSchema,
    }

    impl NamedTool {
        fn arc(name: &str) -> Arc<dyn BaseTool> {
            Arc::new(Self {
                name: name.to_string(),
                args_schema: ArgsSchema::JsonSchema(json!({"type": "object"})),
            })
        }
    }

    #[async_trait]
    impl BaseTool for NamedTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "named test tool"
        }

        fn args_schema(&self) -> Option<&ArgsSchema> {
            Some(&self.args_schema)
        }

        async fn tool_run(
            &self,
            _input: ToolInput,
            _run_manager: Option<&CallbackManagerForToolRun>,
            _config: &RunnableConfig,
        ) -> ToolResult<ToolOutput> {
            Ok(ToolOutput::String(String::new()))
        }
    }

    fn remote_descriptor(name: &str, required: &[&str]) -> WireToolDescriptor {
        let mut d = crate::test_support::bridge_descriptor(name, 2_000);
        d.required_contexts = required.iter().map(|s| (*s).to_string()).collect();
        d
    }

    fn active(key: &str) -> WireActiveContext {
        WireActiveContext {
            key: key.to_string(),
            activated_at: DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            data: json!({}),
        }
    }

    #[test]
    fn build_keeps_remote_tools_whose_contexts_are_active() {
        let catalog = TurnCatalog::build(
            [],
            [remote_descriptor(
                "browser::youtube::get_current_timestamp",
                &["youtube::watch_page"],
            )],
            &[active("youtube::watch_page")],
        )
        .expect("contexts are active");
        assert!(
            catalog
                .get("browser::youtube::get_current_timestamp")
                .is_some()
        );
        assert_eq!(catalog.len(), 1);
    }

    #[test]
    fn build_drops_remote_tools_whose_contexts_are_missing() {
        let catalog = TurnCatalog::build(
            [],
            [remote_descriptor(
                "browser::youtube::get_transcript",
                &["youtube::watch_page", "browser::active_tab"],
            )],
            &[active("youtube::watch_page")],
        )
        .expect("dropping is not an error");
        assert!(catalog.is_empty());
    }

    #[test]
    fn build_detects_collision_between_server_local_and_remote() {
        let err = TurnCatalog::build(
            [NamedTool::arc("firecrawl_search")],
            [remote_descriptor("firecrawl_search", &[])],
            &[],
        )
        .unwrap_err();
        match err {
            CatalogBuildError::NameCollision(name) => assert_eq!(name, "firecrawl_search"),
        }
    }

    #[test]
    fn build_detects_collision_between_two_remotes() {
        let err = TurnCatalog::build(
            [],
            [
                remote_descriptor("browser::dup", &[]),
                remote_descriptor("browser::dup", &[]),
            ],
            &[],
        )
        .unwrap_err();
        match err {
            CatalogBuildError::NameCollision(name) => assert_eq!(name, "browser::dup"),
        }
    }

    #[test]
    fn build_detects_collision_between_two_server_locals() {
        let err = TurnCatalog::build(
            [
                NamedTool::arc("firecrawl_search"),
                NamedTool::arc("firecrawl_search"),
            ],
            [],
            &[],
        )
        .unwrap_err();
        match err {
            CatalogBuildError::NameCollision(name) => assert_eq!(name, "firecrawl_search"),
        }
    }

    #[test]
    fn tool_likes_emits_definition_for_remote_entries() {
        let catalog = TurnCatalog::build(
            [],
            [remote_descriptor(
                "browser::youtube::get_current_timestamp",
                &[],
            )],
            &[],
        )
        .unwrap();
        let likes = catalog.tool_likes();
        assert_eq!(likes.len(), 1);
        match &likes[0] {
            ToolLike::Definition(def) => {
                assert_eq!(def.name, "browser::youtube::get_current_timestamp");
                assert_eq!(def.description, "x");
                assert_eq!(def.parameters, json!({"type": "object"}));
            }
            other => panic!("expected ToolLike::Definition, got {other:?}"),
        }
    }

    #[test]
    fn tool_likes_emits_tool_for_server_local_entries() {
        let catalog = TurnCatalog::build([NamedTool::arc("firecrawl_search")], [], &[]).unwrap();
        let likes = catalog.tool_likes();
        assert_eq!(likes.len(), 1);
        assert!(matches!(&likes[0], ToolLike::Tool(_)));
    }

    /// The first call builds the vector and caches it; the second call
    /// returns the same `Arc`, so the agent-loop forced-synthesis rebind
    /// pays no rebuild cost. Pin pointer-equality to lock the cache.
    #[test]
    fn tool_likes_is_memoized_across_calls() {
        let catalog = TurnCatalog::build(
            [NamedTool::arc("firecrawl_search")],
            [remote_descriptor("browser::dup", &[])],
            &[],
        )
        .unwrap();
        let a = catalog.tool_likes();
        let b = catalog.tool_likes();
        assert!(
            Arc::ptr_eq(&a, &b),
            "tool_likes() must return the same Arc on repeat calls",
        );
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn build_context_system_message_returns_none_when_empty() {
        assert!(build_context_system_message(&[]).is_none());
    }

    #[test]
    fn build_context_system_message_youtube_golden() {
        let ctx = WireActiveContext {
            key: "youtube::watch_page".into(),
            activated_at: DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            data: json!({
                "title": "Tokio async patterns",
                "channel": "ThePrimeagen",
                "duration_seconds": 1122,
                "approximate_timestamp_seconds": 153,
            }),
        };
        let msg = build_context_system_message(&[ctx]).expect("rendered");
        let text = msg
            .content
            .iter()
            .filter_map(|block| match block {
                agent_chain::messages::ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<String>();
        let expected = "Live context from the user's machine:\n\n\
            ## YouTube video (currently playing)\n\
            - Title: Tokio async patterns\n\
            - Channel: ThePrimeagen\n\
            - Duration: 18:42\n\
            - Approximate current playback time: 2:33 (call `browser::youtube::get_current_timestamp` for precise)\n\
            \nTool calls for these contexts are pinned to the specific source the user was on at \
            turn start. If the user closes the tab or navigates away, calls return \
            `context_unavailable` — acknowledge the change to the user rather than retrying.";
        assert_eq!(text, expected);
    }

    #[test]
    fn build_context_system_message_generic_formatter_for_unknown_key() {
        let ctx = WireActiveContext {
            key: "focus::app::vscode".into(),
            activated_at: DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            data: json!({"window_title": "main.rs"}),
        };
        let msg = build_context_system_message(&[ctx]).expect("rendered");
        let text = msg
            .content
            .iter()
            .filter_map(|block| match block {
                agent_chain::messages::ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert!(text.contains("## Context `focus::app::vscode`"));
        assert!(text.contains("\"window_title\": \"main.rs\""));
    }

    #[test]
    fn fmt_hms_renders_hours_when_present() {
        assert_eq!(fmt_hms(0.0), "0:00");
        assert_eq!(fmt_hms(45.0), "0:45");
        assert_eq!(fmt_hms(125.6), "2:06");
        assert_eq!(fmt_hms(3_600.0), "1:00:00");
        assert_eq!(fmt_hms(3_725.0), "1:02:05");
    }

    #[test]
    fn fmt_hms_handles_invalid_inputs() {
        assert_eq!(fmt_hms(-1.0), "0:00");
        assert_eq!(fmt_hms(f64::NAN), "0:00");
        assert_eq!(fmt_hms(f64::INFINITY), "0:00");
    }
}
