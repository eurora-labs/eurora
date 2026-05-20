import type { ObserverSpec } from './types';
import type browser from 'webextension-polyfill';

/**
 * Context key for the generic web-page observer. Keep in sync with the
 * server-side `format_web_page` arm in
 * `be-thread-service::tool_catalog` and the adapter's
 * `requires_context` attribute in `eurora-tools-web`.
 */
export const WEB_PAGE_KEY = 'web::page';

export interface WebPageState {
	tabId: number;
	windowId: number;
	pageUrl: string;
	host: string;
	title: string | null;
}

/**
 * Decide whether `url` is a generic web page worth tracking, and if so
 * return the canonical pieces we care about. Exported for unit tests.
 *
 * The classification rejects every non-`http(s)` scheme so the LLM only
 * ever sees the user's browsing on the open web. `about:blank`,
 * `chrome://settings`, `chrome-extension://…`, `file://…`, `data:…`,
 * `view-source:…`, `javascript:…` — all deactivating.
 */
export function classifyWebUrl(url: string): { pageUrl: string; host: string } | null {
	let parsed: URL;
	try {
		parsed = new URL(url);
	} catch {
		return null;
	}
	if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
		return null;
	}
	return { pageUrl: parsed.toString(), host: parsed.host };
}

export const webObserver: ObserverSpec<WebPageState> = {
	key: WEB_PAGE_KEY,

	classify(tab: browser.Tabs.Tab | undefined): WebPageState | null {
		if (!tab || tab.id === undefined || tab.windowId === undefined || !tab.url) {
			return null;
		}
		const parsed = classifyWebUrl(tab.url);
		if (!parsed) return null;
		return {
			tabId: tab.id,
			windowId: tab.windowId,
			pageUrl: parsed.pageUrl,
			host: parsed.host,
			title: tab.title ?? null,
		};
	},

	sameState(a: WebPageState, b: WebPageState): boolean {
		// Tab + URL identifies the observation. Title is allowed to
		// change without forcing a re-publish — it's a derived field and
		// the desktop already has a copy of it.
		return a.tabId === b.tabId && a.pageUrl === b.pageUrl;
	},

	activatedPayload(state: WebPageState): Record<string, unknown> {
		return {
			url: state.pageUrl,
			host: state.host,
			title: state.title,
			// The content script resolves language from the live document
			// during its bootstrap. Until that lands, surface `null` and
			// rely on the server-side formatter rendering "unknown".
			language: null,
		};
	},

	tabIdOf(state: WebPageState): number {
		return state.tabId;
	},

	originOf(state: WebPageState) {
		return {
			tab_id: state.tabId,
			window_id: `win-${state.windowId}`,
			page_url: state.pageUrl,
		};
	},
};
