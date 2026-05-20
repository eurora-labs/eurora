import { classifyYoutubeUrl, webObserver, youtubeObserver } from './observers';
import browser from 'webextension-polyfill';
import type { ObserverSpec } from './observers';
import type { TabChange, TabStateBus } from './tab-state-bus';
import type { Frame, Payload } from '../content/bindings';

const CONTEXT_ACTIVATED = 'CONTEXT_ACTIVATED';
const CONTEXT_DEACTIVATED = 'CONTEXT_DEACTIVATED';

/**
 * The list of observers driving the context registry. Order is
 * informational only — every spec runs on every tab change, and any
 * number of keys can be simultaneously active (e.g. on a YouTube watch
 * page the model sees both `web::page` and `youtube::watch_page`).
 *
 * To add a new context key (X, Google Docs, …): write a new
 * [`ObserverSpec`] in `observers/` and append it here.
 */
const OBSERVERS: ObserverSpec<unknown>[] = [
	youtubeObserver as ObserverSpec<unknown>,
	webObserver as ObserverSpec<unknown>,
];

/** Map of currently-active state per observer key. */
const active = new Map<string, unknown>();

let nativePort: browser.Runtime.Port | null = null;
let unsubscribe: (() => void) | null = null;

/// Re-exported so the existing `classifyYoutubeUrl` tests keep working
/// without reaching into the observer module directly.
export { classifyYoutubeUrl };
/// Companion export for the generic web observer's URL classifier.
export { classifyWebUrl } from './observers/web';

/// Begin observing focus / URL changes and publishing
/// `CONTEXT_ACTIVATED` / `CONTEXT_DEACTIVATED` events through `port`.
///
/// The bus owns the underlying `chrome.tabs` / `chrome.windows`
/// subscriptions — every consumer in the background page shares one
/// listener set. Replaces any previous registration so the native
/// messenger can call this on every reconnect idempotently.
export function startContextObserver(port: browser.Runtime.Port, bus: TabStateBus): void {
	stopContextObserver();
	nativePort = port;
	unsubscribe = bus.subscribe(onTabChange);
}

/// Stop observing and clear local state. Does not emit deactivations —
/// caller should do that explicitly before tearing down if it wants the
/// desktop to drop entries.
export function stopContextObserver(): void {
	if (unsubscribe) {
		unsubscribe();
		unsubscribe = null;
	}
	nativePort = null;
	active.clear();
}

async function onTabChange(change: TabChange): Promise<void> {
	if (!nativePort) return;

	if (change.cause === 'removed') {
		const removedTabId = change.removedTabId;
		if (removedTabId === undefined) return;
		for (const spec of OBSERVERS) {
			const state = active.get(spec.key);
			if (state !== undefined && spec.tabIdOf(state) === removedTabId) {
				emitDeactivated(spec, state);
				active.delete(spec.key);
			}
		}
		return;
	}

	if (change.cause === 'window-focus' && change.windowId === -1) {
		// `WINDOW_ID_NONE`: the browser lost focus entirely. The user's
		// attention is elsewhere; deactivate every key.
		for (const spec of OBSERVERS) {
			const state = active.get(spec.key);
			if (state !== undefined) {
				emitDeactivated(spec, state);
				active.delete(spec.key);
			}
		}
		return;
	}

	if (change.cause === 'updated') {
		// Only react when something meaningful changed. `status: 'complete'`
		// covers the title settling after a hard navigation; `url` covers
		// SPA navigation between routes without a full page load.
		const changeInfo = change.changeInfo ?? {};
		if (changeInfo.url === undefined && changeInfo.status !== 'complete') return;
		if (change.activeTab && !change.activeTab.active) return;
	}

	const tab = change.activeTab;
	for (const spec of OBSERVERS) {
		const next = spec.classify(tab);
		transitionKey(spec, next);
	}
}

function transitionKey<S>(spec: ObserverSpec<S>, next: S | null): void {
	const previous = active.get(spec.key) as S | undefined;

	if (next === null) {
		if (previous !== undefined) {
			emitDeactivated(spec, previous);
			active.delete(spec.key);
		}
		return;
	}

	if (previous === undefined) {
		emitActivated(spec, next);
		active.set(spec.key, next);
		return;
	}

	if (spec.sameState(previous, next)) {
		// No-op resync — same observable state.
		return;
	}

	// Real transition: different state for the same key. Deactivate the
	// previous, then activate the new, so the desktop sees a clean swap.
	emitDeactivated(spec, previous);
	emitActivated(spec, next);
	active.set(spec.key, next);
}

function emitActivated<S>(spec: ObserverSpec<S>, state: S): void {
	const payload = {
		key: spec.key,
		data: spec.activatedPayload(state),
		// `process_id` is intentionally omitted — the desktop stamps it
		// from the bridge envelope's `app_pid`. See
		// `crates/app/euro-tauri/src/context_registry.rs`.
		origin: { Browser: spec.originOf(state) },
	};
	postEvent(CONTEXT_ACTIVATED, payload);
}

function emitDeactivated<S>(spec: ObserverSpec<S>, state: S): void {
	postEvent(CONTEXT_DEACTIVATED, { key: spec.key, tab_id: spec.tabIdOf(state) });
}

function postEvent(action: string, payload: unknown): void {
	if (!nativePort) return;
	const frame: Frame = {
		kind: {
			Event: {
				action,
				// `payload` is the bridge protocol's inline JSON value
				// — no `JSON.stringify` step. The Rust-side Frame parser
				// captures the raw JSON region verbatim and a typed
				// consumer narrows it (`Payload::deserialize`).
				payload: payload as Payload,
			},
		},
	};
	try {
		nativePort.postMessage(frame);
	} catch (err) {
		console.error(`context-observer: failed to post ${action}:`, err);
	}
}
