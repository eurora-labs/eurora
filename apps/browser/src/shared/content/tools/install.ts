import { cancelInflight } from './invoke';
import browser from 'webextension-polyfill';
import type { Watcher } from './types';

/// Bridge message types the tool framework owns. The background script
/// forwards bridge actions of the same name verbatim onto the active
/// tab; this listener responds.
type ToolMessage =
	| { type: 'LIST_TOOLS' }
	| { type: 'INVOKE_TOOL'; call_id: number; name: string; arguments?: unknown }
	| { type: 'CANCEL_TOOL'; call_id: number };

function isToolMessage(value: unknown): value is ToolMessage {
	if (typeof value !== 'object' || value === null || !('type' in value)) {
		return false;
	}
	const t = (value as { type: unknown }).type;
	return t === 'LIST_TOOLS' || t === 'INVOKE_TOOL' || t === 'CANCEL_TOOL';
}

/// Attach the per-frame `runtime.onMessage` handler that fields the
/// three tool actions and delegates to `watcher`. Called from each site
/// bundle's `main()` after the watcher is constructed — one watcher per
/// frame, no shared registry.
///
/// Non-tool messages (`GET_METADATA`, site-specific messages handled by
/// the bundle's other listeners, etc.) are passed through by returning
/// `undefined` from the listener, which lets sibling listeners reply.
export function installToolHandlers(watcher: Watcher): void {
	browser.runtime.onMessage.addListener(async (message) => {
		if (!isToolMessage(message)) {
			return undefined;
		}

		switch (message.type) {
			case 'LIST_TOOLS':
				return await Promise.resolve({ tools: watcher.listTools() });

			case 'INVOKE_TOOL':
				return await watcher.invoke(message.call_id, message.name, message.arguments);

			case 'CANCEL_TOOL':
				cancelInflight(message.call_id);
				return await Promise.resolve({});
		}
	});
}
