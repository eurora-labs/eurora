import { invokeFrom } from './invoke';
import type { ContextResponse, Tool, Watcher } from './types';
import type { z } from 'zod';

/// Build a `Watcher` whose tool list and context summary are whatever
/// `tools()` / `context()` return at call time. The function form is
/// what lets page-state-dependent surfaces (YouTube `/watch` vs
/// `/feed`) update between calls without a content-script reload —
/// both sources are re-evaluated per request.
///
/// For static surfaces (most sites), `tools` can just close over a
/// constant array. `context` is always required: each site bundle must
/// make an explicit decision about what to say about itself; returning
/// `{ blocks: [] }` is the way to opt out for a given page.
export function watcherFromTools(
	tools: () => readonly Tool<z.ZodTypeAny, unknown>[],
	context: () => ContextResponse,
): Watcher {
	return {
		listTools: () => tools().map((tool) => tool.descriptor),
		getContext: () => context(),
		invoke: async (callId, name, args) => await invokeFrom(tools(), callId, name, args),
	};
}
