import type { ObserverSpec } from './types';
import type browser from 'webextension-polyfill';

/**
 * Context key for the YouTube watch-page observer. Keep in sync with
 * the server-side per-key formatter
 * (`be-thread-service::tool_catalog::format_youtube_watch_page`) and
 * the adapter's `requires_context` attribute in
 * `eurora-tools-youtube`.
 */
export const YOUTUBE_WATCH_KEY = 'youtube::watch_page';

export interface YoutubeWatchState {
	tabId: number;
	windowId: number;
	videoId: string;
	pageUrl: string;
	title: string | null;
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

export const youtubeObserver: ObserverSpec<YoutubeWatchState> = {
	key: YOUTUBE_WATCH_KEY,

	classify(tab: browser.Tabs.Tab | undefined): YoutubeWatchState | null {
		if (!tab || tab.id === undefined || tab.windowId === undefined || !tab.url) {
			return null;
		}
		const parsed = classifyYoutubeUrl(tab.url);
		if (!parsed) return null;
		return {
			tabId: tab.id,
			windowId: tab.windowId,
			videoId: parsed.videoId,
			pageUrl: parsed.pageUrl,
			title: tab.title ?? null,
		};
	},

	sameState(a: YoutubeWatchState, b: YoutubeWatchState): boolean {
		return a.tabId === b.tabId && a.videoId === b.videoId;
	},

	activatedPayload(state: YoutubeWatchState): Record<string, unknown> {
		return {
			video_id: state.videoId,
			title: state.title,
			page_url: state.pageUrl,
		};
	},

	tabIdOf(state: YoutubeWatchState): number {
		return state.tabId;
	},

	originOf(state: YoutubeWatchState) {
		return {
			tab_id: state.tabId,
			window_id: `win-${state.windowId}`,
			page_url: state.pageUrl,
		};
	},
};
