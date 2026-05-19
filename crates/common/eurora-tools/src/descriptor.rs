//! In-process tool descriptor and its conversion to the wire form.
//!
//! The framework form ([`ToolDescriptor`]) is the data the adapter macro
//! emits and the [`crate::Catalog`] indexes: all-`&'static` data plus two
//! `fn() -> &'static schemars::Schema` accessors. It never crosses the
//! WebSocket — [`ToolDescriptor::to_wire`] produces the owned, serializable
//! [`thread_core::WireToolDescriptor`] that does.

use std::time::Duration;

use agent_chain_core::tools::ToolDefinition;
use thread_core::{ToolSource, WireToolDescriptor};

use crate::schema::SchemaFn;

/// Framework-side descriptor for a tool.
///
/// Stored in `&'static [ToolDescriptor]` slices that the macro emits per
/// adapter and that [`crate::Catalog`] indexes by name. The fields
/// intentionally all use `&'static` or `Copy` types so the descriptor
/// itself is trivially `Clone` and can be embedded in `LazyLock`s.
#[derive(Debug, Clone)]
pub struct ToolDescriptor {
    /// Fully-qualified tool name, namespaced with `::`
    /// (e.g. `browser::youtube::get_current_timestamp`). Must be unique
    /// across the whole catalog.
    pub name: &'static str,
    /// Human-readable description shown to the LLM. The macro takes this
    /// from the first paragraph of the trait method's rustdoc.
    pub description: &'static str,
    /// Accessor for the tool's input JSON Schema. Returns a process-wide
    /// cached `&'static schemars::Schema`; see [`crate::schema_of`].
    pub input_schema: SchemaFn,
    /// Accessor for the tool's output JSON Schema. Same caching as
    /// `input_schema`.
    pub output_schema: SchemaFn,
    /// Per-call timeout used by the server's `RemoteToolBus`. Saturating
    /// cast to `u32` milliseconds on the wire (see [`to_wire`]).
    ///
    /// [`to_wire`]: ToolDescriptor::to_wire
    pub timeout: Duration,
    /// Where this tool runs — drives the server-side dispatch decision.
    pub source: ToolSource,
    /// Context keys whose presence is required for this tool to be
    /// advertised in a given turn (e.g. `["youtube::watch_page"]`).
    pub required_contexts: &'static [&'static str],
    /// If `true`, the server must obtain explicit user approval before
    /// dispatching. Not enforced in v1; declared so the protocol is
    /// stable.
    pub requires_user_approval: bool,
}

impl ToolDescriptor {
    /// Produce the owned, serializable wire counterpart.
    ///
    /// Invokes the `input_schema` / `output_schema` accessors and
    /// serialises their results into `serde_json::Value`s embedded in the
    /// wire descriptor. The schemars crate's `Schema` type always
    /// serialises cleanly (it's effectively a wrapper around a JSON value
    /// already), so the `serde_json::to_value` calls are infallible in
    /// practice — `.expect` is the right tool here, not a `Result` return.
    ///
    /// `timeout` is **saturating-cast** to `u32` milliseconds. Anything
    /// over `u32::MAX` ms (~49 days) clamps; tool timeouts shouldn't get
    /// anywhere near that, and saturation keeps the wire shape inside
    /// JavaScript's safe-integer range.
    pub fn to_wire(&self) -> WireToolDescriptor {
        let timeout_ms = u32::try_from(self.timeout.as_millis()).unwrap_or(u32::MAX);
        WireToolDescriptor {
            definition: ToolDefinition {
                name: self.name.to_owned(),
                description: self.description.to_owned(),
                parameters: serde_json::to_value((self.input_schema)())
                    .expect("schemars::Schema must serialize to JSON"),
            },
            output_schema: serde_json::to_value((self.output_schema)())
                .expect("schemars::Schema must serialize to JSON"),
            timeout_ms,
            source: self.source.clone(),
            required_contexts: self
                .required_contexts
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            requires_user_approval: self.requires_user_approval,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::schema_of;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, JsonSchema)]
    struct SampleInput {
        query: String,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    struct SampleOutput {
        result: u32,
    }

    fn sample_descriptor() -> ToolDescriptor {
        const REQUIRED: &[&str] = &["youtube::watch_page"];
        ToolDescriptor {
            name: "browser::youtube::get_current_timestamp",
            description: "Return the user's current playback position.",
            input_schema: schema_of::<SampleInput>,
            output_schema: schema_of::<SampleOutput>,
            timeout: Duration::from_millis(2_000),
            source: ToolSource::Bridge {
                app_kind: "browser".into(),
            },
            required_contexts: REQUIRED,
            requires_user_approval: false,
        }
    }

    #[test]
    fn to_wire_preserves_name_and_description() {
        let wire = sample_descriptor().to_wire();
        assert_eq!(
            wire.definition.name,
            "browser::youtube::get_current_timestamp"
        );
        assert_eq!(
            wire.definition.description,
            "Return the user's current playback position."
        );
    }

    #[test]
    fn to_wire_invokes_schema_accessors() {
        let wire = sample_descriptor().to_wire();
        assert!(wire.definition.parameters.is_object());
        assert!(wire.output_schema.is_object());
        let input_str = serde_json::to_string(&wire.definition.parameters).unwrap();
        let output_str = serde_json::to_string(&wire.output_schema).unwrap();
        assert!(
            input_str.contains("\"query\""),
            "parameters schema should describe the `query` field: {input_str}"
        );
        assert!(
            output_str.contains("\"result\""),
            "output schema should describe the `result` field: {output_str}"
        );
    }

    #[test]
    fn to_wire_converts_timeout_to_milliseconds() {
        let wire = sample_descriptor().to_wire();
        assert_eq!(wire.timeout_ms, 2_000);
    }

    #[test]
    fn to_wire_saturates_oversized_timeout() {
        let mut d = sample_descriptor();
        d.timeout = Duration::MAX;
        let wire = d.to_wire();
        assert_eq!(wire.timeout_ms, u32::MAX);
    }

    #[test]
    fn to_wire_owns_static_strings() {
        const REQUIRED: &[&str] = &["youtube::watch_page", "browser::active_tab"];
        let d = ToolDescriptor {
            name: "browser::test::tool",
            description: "x",
            input_schema: schema_of::<SampleInput>,
            output_schema: schema_of::<SampleOutput>,
            timeout: Duration::from_millis(1),
            source: ToolSource::ClientLocal,
            required_contexts: REQUIRED,
            requires_user_approval: false,
        };
        let wire = d.to_wire();
        assert_eq!(
            wire.required_contexts,
            vec![
                "youtube::watch_page".to_string(),
                "browser::active_tab".to_string()
            ]
        );
    }

    #[test]
    fn to_wire_preserves_source_and_approval_flag() {
        let mut d = sample_descriptor();
        d.requires_user_approval = true;
        let wire = d.to_wire();
        assert!(wire.requires_user_approval);
        assert_eq!(
            wire.source,
            ToolSource::Bridge {
                app_kind: "browser".into()
            }
        );
    }
}
