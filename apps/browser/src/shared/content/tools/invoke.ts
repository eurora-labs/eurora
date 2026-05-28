import { z } from 'zod';
import type { Tool } from './types';
import type { InvokeResponse } from './wire';

/// Per-frame map of in-flight `call_id` → `AbortController`. The only
/// state the tool framework carries — everything else is per-call. The
/// map is keyed by `call_id` rather than tool name because a watcher
/// may legitimately have the same tool running twice concurrently with
/// different args (e.g. parallel `query_selector` calls).
const INFLIGHT = new Map<number, AbortController>();

/// Look up `name` in `tools`, race the handler against a cancellable
/// `AbortController` keyed on `callId`, and map any thrown error onto
/// the appropriate `ToolErrorWire` variant. Returns the wire envelope
/// the desktop side decodes verbatim.
///
/// This is a free function rather than a method on `Watcher` because
/// every watcher's `invoke` boilerplate is identical — composing tools
/// inline and forwarding to this helper keeps watchers to a couple of
/// lines each.
export async function invokeFrom(
	tools: readonly Tool<z.ZodTypeAny, unknown>[],
	callId: number,
	name: string,
	rawArgs: unknown,
): Promise<InvokeResponse> {
	const tool = tools.find((t) => t.descriptor.name === name);
	if (!tool) {
		return {
			err: {
				kind: 'remote',
				code: 404,
				message: `unknown tool \`${name}\``,
			},
		};
	}

	const parsed = tool.argsSchema.safeParse(rawArgs);
	if (!parsed.success) {
		return { err: { kind: 'decode', message: parsed.error.message } };
	}

	const controller = new AbortController();
	INFLIGHT.set(callId, controller);
	try {
		const value = await tool.run(parsed.data, controller.signal);
		return { ok: value };
	} catch (err) {
		if (controller.signal.aborted) {
			return { err: { kind: 'cancelled' } };
		}
		if (err instanceof z.ZodError) {
			return { err: { kind: 'decode', message: err.message } };
		}
		const message = err instanceof Error ? err.message : String(err);
		return { err: { kind: 'adapter', message } };
	} finally {
		INFLIGHT.delete(callId);
	}
}

/// Abort the in-flight call identified by `callId`. No-op if the call
/// has already resolved or was never registered. Idempotent — calling
/// twice for the same id is safe.
export function cancelInflight(callId: number): void {
	const controller = INFLIGHT.get(callId);
	if (!controller) return;
	controller.abort();
	INFLIGHT.delete(callId);
}
