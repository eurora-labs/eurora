import { buildSelectorPath } from '../../extensions/web/selector-path';
import { z } from 'zod';

/// Shape of a single tweet emitted by `list_timeline_tweets` and
/// `get_tweet_thread`. Centralised here so both tools advertise the
/// same JSON schema to the LLM and so any future tweet field lands in
/// one place. `selector_path` is the stable handle the LLM can hand to
/// `web_query_selector` to drill back into the same element.
export const Tweet = z.object({
	selector_path: z.string(),
	text: z.string(),
	author: z.string().nullable(),
	timestamp: z.string().nullable(),
	status_url: z.string().nullable(),
	image_urls: z.array(z.string()),
});

export type TweetT = z.infer<typeof Tweet>;

export type PageKind = 'home' | 'profile' | 'search' | 'notifications' | 'tweet' | 'unsupported';

/// Path prefixes that surface UI we don't model — settings panes, DMs,
/// the composer overlay, etc. Matched before the profile / status
/// regexes so e.g. `/i/flow/...` doesn't get mistaken for a profile.
const UNSUPPORTED_PREFIXES = ['/settings', '/i/', '/messages', '/compose', '/lists', '/bookmarks'];

/// Resolve the page kind for an X.com pathname. Precedence matches the
/// legacy `TwitterParser` routing table (`home`/`search`/`notifications`
/// → `unsupported` prefixes → `/<handle>/status/<id>` → `/<handle>`).
export function getPageKind(pathname: string = window.location.pathname): PageKind {
	if (pathname === '/' || pathname === '/home') return 'home';
	if (pathname.startsWith('/search')) return 'search';
	if (pathname.startsWith('/notifications')) return 'notifications';
	if (UNSUPPORTED_PREFIXES.some((prefix) => pathname.startsWith(prefix))) return 'unsupported';
	if (/^\/[^/]+\/status\/\d+/.test(pathname)) return 'tweet';
	if (/^\/[^/]+\/?$/.test(pathname)) return 'profile';
	return 'unsupported';
}

/// `@handle` for the active profile page, or `null` on any other page.
/// Reads from the path rather than the DOM so it works even before the
/// profile header has rendered.
export function resolveProfileHandle(pathname: string = window.location.pathname): string | null {
	if (getPageKind(pathname) !== 'profile') return null;
	const handle = pathname.split('/').filter(Boolean)[0];
	return handle ? handle : null;
}

/// Active `q=` query for `/search` pages, or `null` everywhere else.
export function resolveSearchQuery(): string | null {
	if (getPageKind() !== 'search') return null;
	const value = new URLSearchParams(window.location.search).get('q');
	return value ? value : null;
}

/// Walk every `article[data-testid="tweet"]` under `root` and project
/// each one into a [`TweetT`]. Articles without rendered tweet text are
/// skipped — they're typically promoted-content placeholders or
/// in-flight skeleton rows.
export function extractTweets(root: ParentNode = document): TweetT[] {
	const articles = root.querySelectorAll('article[data-testid="tweet"]');
	const tweets: TweetT[] = [];
	for (const article of Array.from(articles)) {
		const tweet = projectTweet(article);
		if (tweet) tweets.push(tweet);
	}
	return tweets;
}

function projectTweet(article: Element): TweetT | null {
	const textEl = article.querySelector('[data-testid="tweetText"]');
	const text = textEl?.textContent?.trim();
	if (!text) return null;
	return {
		selector_path: safeSelectorPath(article),
		text,
		author: readAuthor(article),
		timestamp: readTimestamp(article),
		status_url: readStatusUrl(article),
		image_urls: readImageUrls(article),
	};
}

function safeSelectorPath(el: Element): string {
	try {
		return buildSelectorPath(el);
	} catch {
		return '';
	}
}

function readAuthor(article: Element): string | null {
	const candidate = article.querySelector('a[tabindex="-1"][role="link"] span')?.textContent;
	const trimmed = candidate?.trim();
	return trimmed ? trimmed : null;
}

function readTimestamp(article: Element): string | null {
	return article.querySelector('time')?.getAttribute('datetime') ?? null;
}

/// The permalink to the tweet's own status URL — X wraps the `<time>`
/// element in the canonical anchor. Resolved against the document
/// origin so relative `/handle/status/id` hrefs come back fully
/// qualified.
function readStatusUrl(article: Element): string | null {
	const anchor = article.querySelector('time')?.closest('a');
	const href = anchor?.getAttribute('href');
	if (!href) return null;
	try {
		return new URL(href, window.location.href).toString();
	} catch {
		return null;
	}
}

function readImageUrls(article: Element): string[] {
	const imgs = article.querySelectorAll<HTMLImageElement>('[data-testid="tweetPhoto"] img');
	const urls: string[] = [];
	for (const img of Array.from(imgs)) {
		if (img.src) urls.push(img.src);
	}
	return urls;
}

export interface FetchedImage {
	url: string;
	base64: string;
	mime_type: string;
}

/// Fetch a single image URL and convert it to `{ base64, mime_type }`.
/// Returns `null` for any failure (network error, non-image response,
/// missing data-URL match) so callers can report partial success
/// without aborting the whole tool call.
export async function fetchImageAsBase64(url: string): Promise<FetchedImage | null> {
	try {
		const response = await fetch(url);
		if (!response.ok) return null;
		const blob = await response.blob();
		const dataUrl = await blobToDataUrl(blob);
		const match = dataUrl.match(/^data:(image\/[^;]+);base64,(.+)$/);
		if (!match) return null;
		return { url, mime_type: match[1], base64: match[2] };
	} catch {
		return null;
	}
}

async function blobToDataUrl(blob: Blob): Promise<string> {
	return await new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onloadend = () => resolve(reader.result as string);
		reader.onerror = () => reject(reader.error ?? new Error('FileReader failed'));
		reader.readAsDataURL(blob);
	});
}
