import browser from 'webextension-polyfill';
import type { Frame } from '../content/bindings';

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

/// Begin observing focus / URL changes and publishing
/// `CONTEXT_ACTIVATED` / `CONTEXT_DEACTIVATED` events through `port`.
///
/// Replaces any previous port — the native messenger calls this on
/// every reconnect, so attach handlers idempotently.
export function startContextObserver(port: browser.Runtime.Port): void {
	nativePort = port;

	browser.tabs.onActivated.addListener(onTabActivated);
	browser.tabs.onUpdated.addListener(onTabUpdated);
	browser.tabs.onRemoved.addListener(onTabRemoved);
	browser.windows.onFocusChanged.addListener(onWindowFocusChanged);

	resyncFromActiveTab().catch((err) =>
		console.error('context-observer initial sync failed:', err),
	);
}

/// Stop observing and clear local state. Does not emit a deactivation
/// — caller should do that explicitly before tearing down if it wants
/// the desktop to drop the entry.
export function stopContextObserver(): void {
	nativePort = null;
	active = null;

	browser.tabs.onActivated.removeListener(onTabActivated);
	browser.tabs.onUpdated.removeListener(onTabUpdated);
	browser.tabs.onRemoved.removeListener(onTabRemoved);
	browser.windows.onFocusChanged.removeListener(onWindowFocusChanged);
}

async function onTabActivated(_info: browser.Tabs.OnActivatedActiveInfoType): Promise<void> {
	await resyncFromActiveTab();
}

async function onTabUpdated(
	_tabId: number,
	changeInfo: browser.Tabs.OnUpdatedChangeInfoType,
	tab: browser.Tabs.Tab,
): Promise<void> {
	// Only react when something we actually care about changed. The
	// `status: 'complete'` arm covers the title settling after a hard
	// navigation; the `url` arm covers YouTube's SPA navigation between
	// `/watch?v=…` URLs without a full page load.
	if (changeInfo.url === undefined && changeInfo.status !== 'complete') return;
	if (!tab.active) return;
	await resyncFromActiveTab();
}

function onTabRemoved(tabId: number): void {
	if (active && active.tabId === tabId) {
		emitDeactivated(active);
		active = null;
	}
}

async function onWindowFocusChanged(windowId: number): Promise<void> {
	if (windowId === browser.windows.WINDOW_ID_NONE) {
		// The browser lost focus entirely. The user's attention is
		// elsewhere; deactivate so the desktop knows the context is no
		// longer live.
		if (active) {
			emitDeactivated(active);
			active = null;
		}
		return;
	}
	await resyncFromActiveTab();
}

async function resyncFromActiveTab(): Promise<void> {
	if (!nativePort) return;

	let tab: browser.Tabs.Tab | undefined;
	try {
		[tab] = await browser.tabs.query({ active: true, currentWindow: true });
	} catch (err) {
		console.error('context-observer: failed to query active tab:', err);
		return;
	}

	const candidate = classifyTab(tab);
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
				payload: JSON.stringify(payload),
			},
		},
	};
	try {
		nativePort.postMessage(frame);
	} catch (err) {
		console.error(`context-observer: failed to post ${action}:`, err);
	}
}
