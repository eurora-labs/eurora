import type { z } from 'zod';
import { invokeFrom } from './invoke';
import type { Tool, Watcher } from './types';

/// Build a `Watcher` whose tool list is whatever `source()` returns each
/// time `listTools` / `invoke` is called. The function form is what lets
/// page-state-dependent surfaces (YouTube `/watch` vs `/feed`) update
/// between calls without a content-script reload — `source` is
/// re-evaluated per request.
///
/// For static surfaces (most sites), `source` can just close over a
/// constant array: `watcherFromTools(() => webTools)`.
export function watcherFromTools(
	source: () => readonly Tool<z.ZodTypeAny, unknown>[],
): Watcher {
	return {
		listTools: () => source().map((tool) => tool.descriptor),
		invoke: (callId, name, args) => invokeFrom(source(), callId, name, args),
	};
}
