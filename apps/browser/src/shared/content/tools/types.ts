import type { z } from 'zod';
import type { InvokeResponse, WireToolDescriptor } from './wire';

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

	/// Execute one tool call. Matched against the descriptors returned by
	/// `listTools`. `signal` aborts when a `CANCEL_TOOL { call_id }`
	/// arrives from the desktop side.
	invoke(callId: number, name: string, args: unknown): Promise<InvokeResponse>;
}
