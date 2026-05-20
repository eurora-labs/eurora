import browser from 'webextension-polyfill';
import type { TabChange, TabStateBus } from './tab-state-bus';
import type { Frame, Payload } from '../content/bindings';

/// The single `ActiveContext.key` this observer manages. Keep in sync
/// with the server-side per-key formatter
/// (`be-thread-service::tool_catalog::format_youtube_watch_page`) and
/// the adapter's `requires_context` attribute in
/// `eurora-tools-youtube`.
const YOUTUBE_WATCH_KEY = 'youtube::watch_page';

const CONTEXT_ACTIVATED = 'CONTEXT_ACTIVATED';
const CONTEXT_DEACTIVATED = 'CONTEXT_DEACTIVATED';

type WatchPage = {
	tabId: number;
	windowId: number;
	videoId: string;
	pageUrl: string;
	title: string | null;
};

let active: WatchPage | null = null;
let nativePort: browser.Runtime.Port | null = null;
let unsubscribe: (() => void) | null = null;

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

/// Stop observing and clear local state. Does not emit a deactivation
/// — caller should do that explicitly before tearing down if it wants
/// the desktop to drop the entry.
export function stopContextObserver(): void {
	if (unsubscribe) {
		unsubscribe();
		unsubscribe = null;
	}
	nativePort = null;
	active = null;
}

async function onTabChange(change: TabChange): Promise<void> {
	if (!nativePort) return;
	if (change.cause === 'removed') {
		if (active && active.tabId === change.removedTabId) {
			emitDeactivated(active);
			active = null;
		}
		return;
	}
	if (change.cause === 'window-focus' && change.windowId === -1) {
		// `WINDOW_ID_NONE`: the browser lost focus entirely. The user's
		// attention is elsewhere; deactivate so the desktop knows the
		// context is no longer live.
		if (active) {
			emitDeactivated(active);
			active = null;
		}
		return;
	}
	if (change.cause === 'updated') {
		// Only react when something meaningful changed. `status:
		// 'complete'` covers the title settling after a hard navigation;
		// `url` covers YouTube's SPA navigation between `/watch?v=…` URLs
		// without a full page load.
		const changeInfo = change.changeInfo ?? {};
		if (changeInfo.url === undefined && changeInfo.status !== 'complete') return;
		if (change.activeTab && !change.activeTab.active) return;
	}

	const candidate = classifyTab(change.activeTab);
	transition(candidate);
}

function classifyTab(tab: browser.Tabs.Tab | undefined): WatchPage | null {
	if (!tab || tab.id === undefined || tab.windowId === undefined || !tab.url) return null;
	const parsed = classifyYoutubeUrl(tab.url);
	if (!parsed) return null;
	return {
		tabId: tab.id,
		windowId: tab.windowId,
		videoId: parsed.videoId,
		pageUrl: parsed.pageUrl,
		title: tab.title ?? null,
	};
}

/// Decide whether `url` is a YouTube watch page, and if so return the
/// canonical pieces we care about. Exported for unit tests.
export function classifyYoutubeUrl(url: string): { videoId: string; pageUrl: string } | null {
	let parsed: URL;
	try {
		parsed = new URL(url);
	} catch {
		return null;
	}
	const host = parsed.hostname.toLowerCase();
	if (host !== 'www.youtube.com' && host !== 'youtube.com' && host !== 'm.youtube.com') {
		return null;
	}
	if (parsed.pathname !== '/watch') return null;
	const videoId = parsed.searchParams.get('v');
	if (!videoId) return null;
	return { videoId, pageUrl: parsed.toString() };
}

function transition(next: WatchPage | null): void {
	if (next === null) {
		if (active) {
			emitDeactivated(active);
			active = null;
		}
		return;
	}

	if (active === null) {
		emitActivated(next);
		active = next;
		return;
	}

	// Same tab, same video — nothing to publish even if focus toggled
	// in and out.
	if (active.tabId === next.tabId && active.videoId === next.videoId) return;

	// Anything else is a real transition: a different video within the
	// same tab (SPA navigation), or a different tab entirely.
	emitDeactivated(active);
	emitActivated(next);
	active = next;
}

function emitActivated(page: WatchPage): void {
	const payload = {
		key: YOUTUBE_WATCH_KEY,
		data: {
			video_id: page.videoId,
			title: page.title,
			page_url: page.pageUrl,
		},
		// `process_id` is intentionally omitted — the desktop stamps it
		// from the bridge envelope's `app_pid`. See
		// `crates/app/euro-tauri/src/context_registry.rs`.
		origin: {
			Browser: {
				tab_id: page.tabId,
				window_id: `win-${page.windowId}`,
				page_url: page.pageUrl,
			},
		},
	};
	postEvent(CONTEXT_ACTIVATED, payload);
}

function emitDeactivated(page: WatchPage): void {
	postEvent(CONTEXT_DEACTIVATED, { key: YOUTUBE_WATCH_KEY, tab_id: page.tabId });
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
