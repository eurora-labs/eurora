/// Mirror of `thread_core::WireToolDescriptor` / `ToolErrorWire` for the
/// extension. The shapes are pinned by serde golden tests in `thread-core`
/// (`wire_tool_descriptor_flattens_definition_on_the_wire`,
/// `tool_error_wire_all_variants_round_trip`). Update here and in
/// `thread-core` together — specta does not currently emit these into
/// the extension's auto-generated `bindings.ts`.

/// Bridge-side tag for a `WireToolDescriptor.source`. Content-script
/// tools always run via the bridge with `app_kind = "browser"`.
export type ToolSource =
	| { kind: 'bridge'; app_kind: string }
	| { kind: 'client_local' }
	| { kind: 'server_local' }
	| { kind: 'acp' };

/// JSON Schema fragment for tool inputs / outputs. Authored by tools
/// via `zod-to-json-schema`; consumed verbatim by the LLM-side binding.
export type JsonSchemaFragment = Record<string, unknown>;

/// Wire-side tool descriptor. The Rust counterpart is
/// `thread_core::WireToolDescriptor`; `name`/`description`/`parameters`
/// are flattened on the wire from the inner `ToolDefinition`, matching
/// the OpenAI tool-calling shape.
export interface WireToolDescriptor {
	name: string;
	description: string;
	parameters: JsonSchemaFragment;
	output_schema: JsonSchemaFragment;
	timeout_ms: number;
	source: ToolSource;
	required_contexts: string[];
	requires_user_approval: boolean;
}

/// Wire-side tool error. Discriminated by `kind`; rendered into the
/// LLM-visible `ToolResponse.err` envelope verbatim.
export type ToolErrorWire =
	| { kind: 'context_unavailable'; tool: string; reason: string }
	| { kind: 'origin_mismatch'; tool: string; expected: string; got: string }
	| { kind: 'timeout' }
	| { kind: 'cancelled' }
	| { kind: 'transport'; message: string }
	| { kind: 'remote'; code: number; message: string; details?: unknown }
	| { kind: 'decode'; message: string }
	| { kind: 'encode'; message: string }
	| { kind: 'adapter'; message: string };

/// `INVOKE_TOOL` reply envelope. `ok` carries the tool's success value
/// verbatim; `err` carries a `ToolErrorWire` so the LLM sees the
/// original failure instead of a transport-wrapped one.
export type InvokeResponse = { ok: unknown } | { err: ToolErrorWire };

/// `LIST_TOOLS` reply payload.
export interface ListToolsResponse {
	tools: WireToolDescriptor[];
}
