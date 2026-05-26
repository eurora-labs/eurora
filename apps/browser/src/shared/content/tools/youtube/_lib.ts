/// Shared YouTube-page helpers used by the per-page tool slices. Page
/// classification is URL-driven so it works before the DOM has settled;
/// per-page selectors live with the individual tool files.

export type PageKind =
	| 'watch'
	| 'shorts'
	| 'search'
	| 'channel'
	| 'playlist'
	| 'home'
	| 'unsupported';

const HANDLE_PREFIX_RE = /^\/@[^/]+/;
const CHANNEL_PREFIX_RE = /^\/(channel|c|user)\/[^/]+/;
const SHORTS_PREFIX_RE = /^\/shorts\/[^/]+/;

/// Resolve the page kind for a YouTube pathname. Precedence is exact
/// routes first (`/watch`, `/results`, `/playlist`), then path-prefix
/// routes (`/shorts/...`, `/@handle`, `/channel/...`), then the home
/// block (`/`, `/feed/*`), then `unsupported`.
export function getPageKind(pathname: string = window.location.pathname): PageKind {
	if (pathname === '/watch') return 'watch';
	if (pathname === '/results') return 'search';
	if (pathname === '/playlist') return 'playlist';
	if (SHORTS_PREFIX_RE.test(pathname)) return 'shorts';
	if (HANDLE_PREFIX_RE.test(pathname) || CHANNEL_PREFIX_RE.test(pathname)) return 'channel';
	if (pathname === '/' || pathname.startsWith('/feed/')) return 'home';
	return 'unsupported';
}

/// Watch-page `?v=` value, or `null` on any other page. URL-driven so
/// the watch-page tools can resolve the id before the player has loaded.
export function resolveWatchVideoId(search: string = window.location.search): string | null {
	return new URLSearchParams(search).get('v');
}

/// Shorts-page video id (the segment after `/shorts/`), or `null`
/// elsewhere.
export function resolveShortsVideoId(pathname: string = window.location.pathname): string | null {
	const match = pathname.match(/^\/shorts\/([^/?#]+)/);
	return match ? match[1] : null;
}

/// Active media video id from either page kind, or `null` when neither
/// URL form is on screen. Tools that work on both watch and shorts call
/// this; page-kind dispatch in the watcher decides which kinds they
/// actually run on.
export function getCurrentVideoId(): string | null {
	const kind = getPageKind();
	if (kind === 'watch') return resolveWatchVideoId();
	if (kind === 'shorts') return resolveShortsVideoId();
	return null;
}

export function requireCurrentVideoId(): string {
	const videoId = getCurrentVideoId();
	if (videoId === null) {
		throw new Error('no YouTube video id resolvable from the current URL');
	}
	return videoId;
}

/// Playlist id from the URL's `list` parameter. Present on `/playlist`
/// pages and on watch pages opened with `&list=...`; `null` elsewhere.
export function resolvePlaylistId(search: string = window.location.search): string | null {
	return new URLSearchParams(search).get('list');
}

/// `@handle` from `/@handle/...` URLs (with the `@` preserved), or
/// `null` on legacy `/channel/<id>` and `/c/<name>` forms.
export function resolveChannelHandle(pathname: string = window.location.pathname): string | null {
	const match = pathname.match(/^\/@([^/]+)/);
	return match ? `@${match[1]}` : null;
}

/// Channel UCID from `/channel/<id>/...` URLs, or `null` elsewhere.
/// Distinct from [`resolveChannelHandle`] because handle and id are
/// separate concepts in YouTube's data model — handle is human-readable
/// and mutable, id is stable.
export function resolveChannelId(pathname: string = window.location.pathname): string | null {
	const match = pathname.match(/^\/channel\/([^/]+)/);
	return match ? match[1] : null;
}

/// `search_query` from `/results?search_query=...`, or `null` elsewhere.
export function resolveSearchQuery(search: string = window.location.search): string | null {
	return new URLSearchParams(search).get('search_query');
}

/// Resolve the `<video>` element. Throws when the page has no player or
/// the player hasn't loaded enough data to read `currentTime` /
/// `duration` (`readyState === 0`).
export function requirePlayer(): HTMLVideoElement {
	const player = document.querySelector<HTMLVideoElement>('video.html5-main-video');
	if (!player) throw new Error('no YouTube player element on the page');
	if (player.readyState === 0) throw new Error('YouTube player not ready');
	return player;
}

/// Best-effort sibling of [`requirePlayer`] that returns `null` instead
/// of throwing when the player isn't on the page yet or hasn't loaded.
/// Used by surfaces (e.g. the per-turn context summary) that must not
/// fail just because the user happened to navigate between videos.
export function readPlayerTime(): { currentTime: number; duration: number | null } | null {
	const player = document.querySelector<HTMLVideoElement>('video.html5-main-video');
	if (!player || player.readyState === 0) return null;
	return {
		currentTime: player.currentTime,
		duration: Number.isFinite(player.duration) ? player.duration : null,
	};
}

/// Parse a YouTube-style timestamp string (`M:SS` or `H:MM:SS`) into
/// seconds. Returns `null` for unrecognised input. Used by the chapter,
/// recommendation, and search-result tools to convert the visible
/// duration strings into machine-readable values.
export function parseHmsTimestamp(text: string): number | null {
	const parts = text.split(':').map((p) => parseInt(p, 10));
	if (parts.length < 2 || parts.length > 3) return null;
	if (parts.some((p) => !Number.isFinite(p))) return null;
	if (parts.length === 2) return parts[0] * 60 + parts[1];
	return parts[0] * 3_600 + parts[1] * 60 + parts[2];
}

/// Extract the `?v=...` value from a YouTube URL (relative or absolute).
/// Returns `null` for non-watch URLs and on parse failure.
export function videoIdFromUrl(url: string): string | null {
	try {
		return new URL(url, window.location.href).searchParams.get('v');
	} catch {
		return null;
	}
}
