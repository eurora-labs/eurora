import { Tweet, extractTweets, getPageKind } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	main_tweet: Tweet.nullable(),
	replies: z.array(Tweet),
});

type Result = z.infer<typeof Out>;

export async function executeGetTweetThread(): Promise<Result> {
	if (getPageKind() !== 'tweet') {
		throw new Error(
			'twitter_get_tweet_thread can only be called on a /<handle>/status/<id> page',
		);
	}
	const all = extractTweets();
	const [main, ...replies] = all;
	return {
		main_tweet: main ?? null,
		replies,
	};
}

export const getTweetThread: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'twitter_get_tweet_thread',
		description:
			'On a `/<handle>/status/<id>` page, return the main tweet (first article in DOM order) plus the visible replies as separate fields. Each entry carries the same shape as `twitter_list_timeline_tweets`. Fails when called on any other X.com page — use `twitter_get_page_context` first if unsure.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 3_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeGetTweetThread();
	},
};
