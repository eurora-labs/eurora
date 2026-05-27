/// Re-exports the wire types the e2e suite asserts against. Importing
/// from `@eurora/browser/tools` keeps the test types pinned to the
/// production tool surface — a rename or shape change in a tool ripples
/// straight into a compile error here, which is the whole point of
/// owning these tests against the same source of truth.

export type {
	ContextResponse,
	InvokeResponse,
	ListToolsResponse,
	Tool,
	ToolErrorWire,
	WireToolDescriptor,
} from '@eurora/browser/tools';

export type { TranscriptSnippet } from '@eurora/browser/transcript';

/// Convenience helper: pull the runtime result type out of a `Tool`
/// without the consumer having to spell out `Awaited<ReturnType<...>>`
/// each time. Used by specs that want to type the `invokeTool` return
/// directly against the production output schema.
///
/// Structural match against `run` rather than `Tool<Args, Out>` so the
/// e2e package doesn't need a direct dependency on `zod` (whose
/// `ZodTypeAny` would otherwise appear in the parameter list).
export type ToolResult<T> = T extends { run: (...args: never[]) => Promise<infer R> } ? R : never;
