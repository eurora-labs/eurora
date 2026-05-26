import { cancelInflight } from './invoke';
import browser from 'webextension-polyfill';
import type { Watcher } from './types';

/// Bridge message types the tool framework owns. The background script
/// forwards bridge actions of the same name verbatim onto the active
/// tab; this listener responds.
type ToolMessage =
	| { type: 'LIST_TOOLS' }
	| { type: 'GET_CONTEXT' }
	| { type: 'INVOKE_TOOL'; call_id: number; name: string; arguments?: unknown }
	| { type: 'CANCEL_TOOL'; call_id: number };

function isToolMessage(value: unknown): value is ToolMessage {
	if (typeof value !== 'object' || value === null || !('type' in value)) {
		return false;
	}
	const t = (value as { type: unknown }).type;
	return t === 'LIST_TOOLS' || t === 'GET_CONTEXT' || t === 'INVOKE_TOOL' || t === 'CANCEL_TOOL';
}

/// Attach the per-frame `runtime.onMessage` handler that fields the
/// four tool actions and delegates to `watcher`. Called from each site
/// bundle's `main()` after the watcher is constructed — one watcher per
/// frame, no shared registry.
///
/// The listener is intentionally NOT `async`: an async function returns
/// `Promise<undefined>` for the early-out paths, and webextension-polyfill
/// treats any returned thenable as "I'm replying asynchronously" — which
/// means a non-matching listener would still claim ownership of the
/// message and respond with `undefined`, stealing the reply from sibling
/// listeners (`_common`'s `GET_METADATA` handler, future per-site
/// listeners). Returning a bare `undefined` for non-tool messages lets the
/// polyfill see `result === undefined`, return `false` to Chrome, and
/// fall through to the next registered listener. Only the matching cases
/// return a Promise.
export function installToolHandlers(watcher: Watcher): void {
	// eslint-disable-next-line @typescript-eslint/promise-function-async -- listener must be sync so the polyfill sees a bare `undefined` return for unhandled messages and falls through to sibling listeners; an async listener would wrap that in `Promise<undefined>` and claim ownership of every message.
	browser.runtime.onMessage.addListener((message) => {
		if (!isToolMessage(message)) {
			return undefined;
		}

		switch (message.type) {
			case 'LIST_TOOLS':
				return Promise.resolve({ tools: watcher.listTools() });

			case 'GET_CONTEXT':
				return Promise.resolve(watcher.getContext());

			case 'INVOKE_TOOL':
				return watcher.invoke(message.call_id, message.name, message.arguments);

			case 'CANCEL_TOOL':
				cancelInflight(message.call_id);
				return Promise.resolve({});
		}
	});
}
