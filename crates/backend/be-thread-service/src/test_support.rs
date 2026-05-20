//! Shared test fixtures for `be-thread-service`.
//!
//! `WireToolDescriptor` was being rebuilt by hand in four places — the
//! agent-loop, tool-catalog, chat-handler, and remote-bus test modules —
//! each with slightly different field defaults and the same boilerplate
//! tool definition. This module is the one place those tests reach for a
//! descriptor; downstream test code only customises what it cares about.

use agent_chain_core::tools::ToolDefinition;
use serde_json::json;
use thread_core::{ToolSource, WireToolDescriptor};

/// Build a `WireToolDescriptor` with the most common shape used in tests:
/// a `bridge(browser)` source, an empty input/output schema, and no
/// required contexts. Caller overrides via the returned-builder pattern
/// are intentionally absent — every call site only needs to tweak a small
/// set of fields, and inlining those tweaks at the call site is clearer
/// than threading them through a builder.
pub fn bridge_descriptor(name: &str, timeout_ms: u32) -> WireToolDescriptor {
    WireToolDescriptor {
        definition: ToolDefinition {
            name: name.to_string(),
            description: "x".to_string(),
            parameters: json!({"type": "object"}),
        },
        output_schema: json!({"type": "object"}),
        timeout_ms,
        source: ToolSource::Bridge {
            app_kind: "browser".to_string(),
        },
        required_contexts: Vec::new(),
        requires_user_approval: false,
    }
}
