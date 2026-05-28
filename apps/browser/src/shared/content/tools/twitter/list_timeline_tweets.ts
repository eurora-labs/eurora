import { Tweet, extractTweets } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const DEFAULT_LIMIT = 50;
const HARD_LIMIT = 200;

const Args = z
	.object({
		limit: z.number().int().positive().optional(),
	})
	.strict();

const Out = z.object({
	tweets: z.array(Tweet),
	total: z.number().int().nonnegative(),
});

type Result = z.infer<typeof Out>;

export async function executeListTimelineTweets(args: z.infer<typeof Args>): Promise<Result> {
	const limit = Math.min(args.limit ?? DEFAULT_LIMIT, HARD_LIMIT);
	const all = extractTweets();
	return {
		tweets: all.slice(0, limit),
		total: all.length,
	};
}

export const listTimelineTweets: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'twitter_list_timeline_tweets',
		description:
			"Return the tweets currently rendered in the active X.com timeline (home feed, profile, search results, or notifications). Each entry carries text, author handle, ISO timestamp, the tweet's own status URL, image URLs (fetch separately with `twitter_fetch_tweet_images` if you need the bytes), and a `selector_path` you can hand to `web_query_selector` to drill back into the same `<article>`. `total` is the pre-`limit` rendered count so the model can opt into a higher limit when worthwhile.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 3_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return await executeListTimelineTweets(args);
	},
};
