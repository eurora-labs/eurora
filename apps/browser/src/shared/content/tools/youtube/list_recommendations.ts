import { parseHmsTimestamp, requireCurrentVideoId, videoIdFromUrl } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const DEFAULT_LIMIT = 10;
const HARD_LIMIT = 50;

const Args = z
	.object({
		limit: z.number().int().positive().optional(),
	})
	.strict();

const Recommendation = z.object({
	video_id: z.string().nullable(),
	title: z.string(),
	channel: z.string().nullable(),
	length_seconds: z.number().nonnegative().nullable(),
	url: z.string(),
});

const Out = z.object({
	video_id: z.string(),
	items: z.array(Recommendation),
	total: z.number().int().nonnegative(),
});

type Result = z.infer<typeof Out>;
type RecommendationT = z.infer<typeof Recommendation>;

function readLengthSeconds(card: Element): number | null {
	const lengthText = card
		.querySelector(
			'ytd-thumbnail-overlay-time-status-renderer #text, ' +
				'ytd-thumbnail-overlay-time-status-renderer .badge-shape-wiz__text, ' +
				'.badge-shape-wiz__text',
		)
		?.textContent?.trim();
	return lengthText ? parseHmsTimestamp(lengthText) : null;
}

function projectRecommendation(card: Element): RecommendationT | null {
	const link = card.querySelector<HTMLAnchorElement>('a#thumbnail');
	const href = link?.href;
	if (!href) return null;
	const title = card.querySelector('#video-title')?.textContent?.trim();
	if (!title) return null;
	return {
		video_id: videoIdFromUrl(href),
		title,
		channel: card.querySelector('ytd-channel-name')?.textContent?.trim() || null,
		length_seconds: readLengthSeconds(card),
		url: href,
	};
}

export async function executeListRecommendations(args: z.infer<typeof Args>): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const limit = Math.min(args.limit ?? DEFAULT_LIMIT, HARD_LIMIT);
	const cards = document.querySelectorAll(
		'ytd-watch-next-secondary-results-renderer ytd-compact-video-renderer, ' +
			'#related ytd-compact-video-renderer',
	);
	const items: RecommendationT[] = [];
	for (const card of Array.from(cards)) {
		if (items.length >= limit) break;
		const projected = projectRecommendation(card);
		if (projected) items.push(projected);
	}
	return {
		video_id: videoId,
		items,
		total: cards.length,
	};
}

export const listRecommendations: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_list_recommendations',
		description:
			"Return the related / 'Up next' videos rendered in the secondary column on the active YouTube watch page. Each item carries `video_id` (when extractable from the link), `title`, `channel`, `length_seconds`, and the absolute `url`. `total` is the pre-`limit` rendered count so the model can opt into a higher limit when worthwhile.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 3_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return await executeListRecommendations(args);
	},
};
