import { fetchImageAsBase64 } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const HARD_LIMIT = 16;

const Args = z
	.object({
		urls: z.array(z.string().url()).min(1).max(HARD_LIMIT),
	})
	.strict();

const FetchedImage = z.object({
	url: z.string(),
	base64: z.string(),
	mime_type: z.string(),
});

const FailedImage = z.object({
	url: z.string(),
	error: z.string(),
});

const Out = z.object({
	images: z.array(FetchedImage),
	failures: z.array(FailedImage),
});

type Result = z.infer<typeof Out>;

export async function executeFetchTweetImages(args: z.infer<typeof Args>): Promise<Result> {
	const settled = await Promise.all(
		args.urls.map(async (url) => {
			try {
				const image = await fetchImageAsBase64(url);
				if (!image) {
					return {
						kind: 'fail' as const,
						url,
						error: 'image fetch returned no decodable data',
					};
				}
				return { kind: 'ok' as const, image };
			} catch (err) {
				const message = err instanceof Error ? err.message : String(err);
				return { kind: 'fail' as const, url, error: message };
			}
		}),
	);

	const images: z.infer<typeof FetchedImage>[] = [];
	const failures: z.infer<typeof FailedImage>[] = [];
	for (const entry of settled) {
		if (entry.kind === 'ok') {
			images.push(entry.image);
		} else {
			failures.push({ url: entry.url, error: entry.error });
		}
	}
	return { images, failures };
}

export const fetchTweetImages: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'twitter_fetch_tweet_images',
		description:
			'Lazily fetch one or more image URLs (as surfaced by `twitter_list_timeline_tweets` / `twitter_get_tweet_thread`) and return their bytes as base64 plus mime type. Failures are reported per-URL in `failures` so a partial result still comes back when one image 404s or is CORS-blocked. Cap of 16 URLs per call to keep the response bounded.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 15_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return await executeFetchTweetImages(args);
	},
};
