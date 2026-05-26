import { getPageKind, parseHmsTimestamp, resolveSearchQuery, videoIdFromUrl } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const DEFAULT_LIMIT = 20;
const HARD_LIMIT = 100;

const Args = z
	.object({
		limit: z.number().int().positive().optional(),
	})
	.strict();

const SearchItem = z.object({
	kind: z.enum(['video', 'channel', 'playlist']),
	video_id: z.string().nullable(),
	title: z.string(),
	channel: z.string().nullable(),
	length_seconds: z.number().nonnegative().nullable(),
	url: z.string(),
});

const Out = z.object({
	query: z.string().nullable(),
	items: z.array(SearchItem),
	total: z.number().int().nonnegative(),
});

type Result = z.infer<typeof Out>;
type SearchItemT = z.infer<typeof SearchItem>;

function readLengthSeconds(card: Element): number | null {
	const text = card
		.querySelector(
			'ytd-thumbnail-overlay-time-status-renderer #text, ' +
				'ytd-thumbnail-overlay-time-status-renderer .badge-shape-wiz__text, ' +
				'.badge-shape-wiz__text',
		)
		?.textContent?.trim();
	return text ? parseHmsTimestamp(text) : null;
}

function projectVideo(card: Element): SearchItemT | null {
	const link = card.querySelector<HTMLAnchorElement>('a#video-title');
	const href = link?.href;
	const title =
		link?.textContent?.trim() || card.querySelector('#video-title')?.textContent?.trim();
	if (!href || !title) return null;
	return {
		kind: 'video',
		video_id: videoIdFromUrl(href),
		title,
		channel: card.querySelector('ytd-channel-name')?.textContent?.trim() || null,
		length_seconds: readLengthSeconds(card),
		url: href,
	};
}

function projectChannel(card: Element): SearchItemT | null {
	const link = card.querySelector<HTMLAnchorElement>('a');
	const href = link?.href;
	const title = card.querySelector('ytd-channel-name')?.textContent?.trim();
	if (!href || !title) return null;
	return {
		kind: 'channel',
		video_id: null,
		title,
		channel: null,
		length_seconds: null,
		url: href,
	};
}

function projectPlaylist(card: Element): SearchItemT | null {
	const link = card.querySelector<HTMLAnchorElement>('a');
	const href = link?.href;
	const title =
		card.querySelector('#video-title')?.textContent?.trim() ||
		card.querySelector('h3')?.textContent?.trim();
	if (!href || !title) return null;
	return {
		kind: 'playlist',
		video_id: null,
		title,
		channel: card.querySelector('ytd-channel-name')?.textContent?.trim() || null,
		length_seconds: null,
		url: href,
	};
}

export async function executeListSearchResults(args: z.infer<typeof Args>): Promise<Result> {
	if (getPageKind() !== 'search') {
		throw new Error('youtube_list_search_results can only be called on /results pages');
	}

	const limit = Math.min(args.limit ?? DEFAULT_LIMIT, HARD_LIMIT);
	const videoCards = document.querySelectorAll('ytd-video-renderer');
	const channelCards = document.querySelectorAll('ytd-channel-renderer');
	const playlistCards = document.querySelectorAll(
		'ytd-playlist-renderer, ytd-radio-renderer, ytd-show-renderer',
	);

	const items: SearchItemT[] = [];
	const observed = videoCards.length + channelCards.length + playlistCards.length;

	for (const card of Array.from(videoCards)) {
		if (items.length >= limit) break;
		const projected = projectVideo(card);
		if (projected) items.push(projected);
	}
	for (const card of Array.from(channelCards)) {
		if (items.length >= limit) break;
		const projected = projectChannel(card);
		if (projected) items.push(projected);
	}
	for (const card of Array.from(playlistCards)) {
		if (items.length >= limit) break;
		const projected = projectPlaylist(card);
		if (projected) items.push(projected);
	}

	return {
		query: resolveSearchQuery(),
		items,
		total: observed,
	};
}

export const listSearchResults: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_list_search_results',
		description:
			'Return the search results visible on `/results` — videos, channels, and playlists each shaped uniformly with `kind`, `title`, `url`, and where applicable `video_id`, `channel`, and `length_seconds`. `total` counts everything observed before the per-call `limit` was applied. Fails when called on any other YouTube page — use `youtube_get_page_context` first if unsure.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 3_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return await executeListSearchResults(args);
	},
};
