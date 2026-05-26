import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { textContext } from '../../../shared/content/tools/context';
import { installToolHandlers } from '../../../shared/content/tools/install';
import { webTools } from '../../../shared/content/tools/web';
import {
	getPageKind,
	readPlayerTime,
	resolveChannelHandle,
	resolveSearchQuery,
	resolveYoutubeTools,
} from '../../../shared/content/tools/youtube';

let initialized = false;

/// Strip YouTube's `" - YouTube"` suffix so the video title we report
/// reads cleanly. `document.title` lags behind SPA navigation by a
/// frame or two but settles before the model has a chance to act on it.
function watchPageTitle(): string {
	return document.title.replace(/ - YouTube$/, '').trim();
}

function pad2(n: number): string {
	return n.toString().padStart(2, '0');
}

/// Format a non-negative number of seconds as `H:MM:SS` (or `M:SS` when
/// under an hour). Mirrors the desktop-side `fmt_hms` helper so the
/// context message format stays consistent across the two systems.
function formatHms(seconds: number): string {
	if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
	const total = Math.round(seconds);
	const hours = Math.floor(total / 3_600);
	const minutes = Math.floor((total % 3_600) / 60);
	const secs = total % 60;
	return hours > 0 ? `${hours}:${pad2(minutes)}:${pad2(secs)}` : `${minutes}:${pad2(secs)}`;
}

function describeWatch(): string {
	const title = watchPageTitle();
	const time = readPlayerTime();
	const titleClause = title ? ` titled "${title}"` : '';
	if (time === null) {
		return `The user is currently watching a YouTube video${titleClause}.`;
	}
	const stamp = formatHms(time.currentTime);
	if (time.duration !== null) {
		return `The user is currently watching a YouTube video${titleClause} at timestamp ${stamp} of ${formatHms(time.duration)}.`;
	}
	return `The user is currently watching a YouTube video${titleClause} at timestamp ${stamp}.`;
}

function describeSearch(): string {
	const query = resolveSearchQuery();
	return query
		? `The user is searching YouTube for "${query}".`
		: 'The user is searching YouTube.';
}

function describeChannel(): string {
	const handle = resolveChannelHandle();
	return handle
		? `The user is currently viewing the YouTube channel ${handle}.`
		: 'The user is currently viewing a YouTube channel.';
}

/// Per-page summary for the YouTube bundle. Mirrors the routing in
/// `resolveYoutubeTools` so the wording stays in lockstep with the tool
/// surface the LLM also sees.
function describeYoutube(): string {
	const kind = getPageKind();
	switch (kind) {
		case 'watch':
			return describeWatch();
		case 'shorts':
			return 'The user is browsing YouTube Shorts.';
		case 'search':
			return describeSearch();
		case 'channel':
			return describeChannel();
		case 'playlist':
			return 'The user is currently viewing a YouTube playlist.';
		case 'home':
		case 'unsupported':
			return 'The user is browsing YouTube.';
	}
}

/// YouTube content-script bundle. Surfaces the generic web tools
/// alongside the YouTube-specific tools appropriate for the current
/// page — `resolveYoutubeTools` is re-evaluated per `LIST_TOOLS` call so
/// SPA navigation between e.g. `/watch` and `/results` flips the surface
/// without a content-script reload.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(
		watcherFromTools(
			() => [...webTools, ...resolveYoutubeTools()],
			() => textContext(describeYoutube()),
		),
	);
}
