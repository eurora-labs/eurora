import type { InvokeResponse, WireToolDescriptor } from './wire';
import type { ContentBlock } from '@eurora/shared/bindings/thread';
import type { z } from 'zod';

/// One LLM-facing tool: descriptor for advertising, schemas for runtime
/// argument validation and (optional) return validation, plus the
/// handler that does the actual work.
///
/// The descriptor's `parameters` / `output_schema` are typically derived
/// from `argsSchema` / `outputSchema` via `zod-to-json-schema`; the
/// indirection lets tool authors keep one source of truth per shape and
/// also keeps the runtime parsed-args narrowed to `z.infer<Args>` inside
/// `run`.
export interface Tool<Args extends z.ZodTypeAny, Out> {
	readonly descriptor: WireToolDescriptor;
	readonly argsSchema: Args;
	run(args: z.infer<Args>, signal: AbortSignal): Promise<Out>;
}

/// Reply payload for the `GET_CONTEXT` bridge action.
///
/// The wire shape stays in sync with the desktop-side decoder in
/// `crates/app/euro-activity/src/strategies/browser.rs`'s
/// `GetContextPayload`: a flat object holding the list of
/// [`ContentBlock`]s the per-site watcher wants surfaced this turn.
/// An empty array is fine — it just means "nothing meaningful to say".
export interface ContextResponse {
	blocks: ContentBlock[];
}

/// Per-page tool surface. Exactly one watcher is loaded per content-script
/// frame (the bundle's `index.ts` constructs it and calls
/// `installToolHandlers`). The watcher composes its tool list however it
/// likes — spreading reusable `webTools`, conditionally adding
/// site-specific ones based on `location`, or replacing the entire set
/// for a site that doesn't make sense to surface generic web tools on.
export interface Watcher {
	/// Tools the LLM should see right now. Re-evaluated per `LIST_TOOLS`
	/// call so page-state-dependent surfaces (YouTube `/watch` vs
	/// `/feed`) update without a content-script reload.
	listTools(): WireToolDescriptor[];

	/// Short curated summary of what the user is doing on this page,
	/// returned as content blocks the desktop pipes into the next chat
	/// turn (e.g. `"The user is currently watching a video titled X"`).
	/// Re-evaluated per `GET_CONTEXT` call so SPA navigation updates the
	/// wording without a content-script reload.
	getContext(): ContextResponse;

	/// Execute one tool call. Matched against the descriptors returned by
	/// `listTools`. `signal` aborts when a `CANCEL_TOOL { call_id }`
	/// arrives from the desktop side.
	invoke(callId: number, name: string, args: unknown): Promise<InvokeResponse>;
}
