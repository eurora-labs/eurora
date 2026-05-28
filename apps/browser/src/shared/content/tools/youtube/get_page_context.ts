import {
	getPageKind,
	resolveChannelHandle,
	resolveChannelId,
	resolvePlaylistId,
	resolveSearchQuery,
	resolveShortsVideoId,
	resolveWatchVideoId,
	type PageKind,
} from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	kind: z.enum(['watch', 'shorts', 'search', 'channel', 'playlist', 'home', 'unsupported']),
	url: z.string(),
	video_id: z.string().nullable(),
	playlist_id: z.string().nullable(),
	channel_handle: z.string().nullable(),
	channel_id: z.string().nullable(),
	search_query: z.string().nullable(),
});

type Result = z.infer<typeof Out>;

function videoIdForKind(kind: PageKind): string | null {
	if (kind === 'watch') return resolveWatchVideoId();
	if (kind === 'shorts') return resolveShortsVideoId();
	return null;
}

export async function executeGetPageContext(): Promise<Result> {
	const kind = getPageKind();
	return {
		kind,
		url: window.location.href,
		video_id: videoIdForKind(kind),
		playlist_id: resolvePlaylistId(),
		channel_handle: resolveChannelHandle(),
		channel_id: resolveChannelId(),
		search_query: kind === 'search' ? resolveSearchQuery() : null,
	};
}

export const getPageContext: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_get_page_context',
		description:
			'Identify which kind of YouTube page the user is currently on — one of `watch`, `shorts`, `search`, `channel`, `playlist`, `home`, or `unsupported` — along with URL-derived metadata: `video_id` on watch/shorts pages, `playlist_id` when a list is attached, `channel_handle` / `channel_id` on channel pages, and `search_query` on `/results`. Call this before deciding whether `youtube_get_transcript`, `youtube_list_search_results`, etc. are the right next step.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 1_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeGetPageContext();
	},
};
