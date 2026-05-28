import { getPageKind, resolveProfileHandle, resolveSearchQuery, type PageKind } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	kind: z.enum(['home', 'profile', 'search', 'notifications', 'tweet', 'unsupported']),
	url: z.string(),
	profile_handle: z.string().nullable(),
	search_query: z.string().nullable(),
});

type Result = z.infer<typeof Out>;

export async function executeGetPageContext(): Promise<Result> {
	const kind: PageKind = getPageKind();
	return {
		kind,
		url: window.location.href,
		profile_handle: resolveProfileHandle(),
		search_query: resolveSearchQuery(),
	};
}

export const getPageContext: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'twitter_get_page_context',
		description:
			'Identify which kind of X.com page the user is currently looking at — one of `home`, `profile`, `search`, `notifications`, `tweet`, or `unsupported` — along with URL-derived metadata: `profile_handle` on profile pages and `search_query` on search pages. Call this before deciding whether `twitter_list_timeline_tweets` or `twitter_get_tweet_thread` is the right next step.',
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
