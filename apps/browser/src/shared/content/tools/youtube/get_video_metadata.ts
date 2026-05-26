import { requireCurrentVideoId } from './_lib';
import { READABILITY_BODY_CAP, clampString } from '../../extensions/web/truncation';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	video_id: z.string(),
	title: z.string().nullable(),
	channel_name: z.string().nullable(),
	channel_handle: z.string().nullable(),
	channel_url: z.string().nullable(),
	published_at: z.string().nullable(),
	view_count: z.number().int().nonnegative().nullable(),
	like_count: z.number().int().nonnegative().nullable(),
	description: z.string(),
	description_truncated: z.boolean(),
});

type Result = z.infer<typeof Out>;

function nonEmpty(value: string | null | undefined): string | null {
	if (!value) return null;
	const trimmed = value.trim();
	return trimmed.length > 0 ? trimmed : null;
}

/// `ytd-player-microformat-renderer` is YouTube's hidden schema.org
/// summary of the active video — far more stable than the visible DOM
/// because it backs the page's structured metadata, not the UI.
function readMicroformatAttr(selector: string, attr: 'content' | 'href'): string | null {
	const el = document.querySelector(`ytd-player-microformat-renderer ${selector}`);
	return nonEmpty(el?.getAttribute(attr));
}

function parseDigits(value: string | null): number | null {
	if (value === null) return null;
	const digits = value.replace(/[^\d]/g, '');
	if (!digits) return null;
	const n = parseInt(digits, 10);
	return Number.isFinite(n) ? n : null;
}

function readTitle(): string | null {
	return (
		readMicroformatAttr('meta[itemprop="name"]', 'content') ??
		nonEmpty(
			document.querySelector('ytd-watch-metadata h1, h1.ytd-watch-metadata')?.textContent,
		) ??
		nonEmpty(document.title.replace(/ - YouTube$/, ''))
	);
}

function readChannelUrl(): string | null {
	const fromMicroformat = readMicroformatAttr('link[itemprop="url"]', 'href');
	if (fromMicroformat) return fromMicroformat;
	const anchor = document.querySelector<HTMLAnchorElement>(
		'#owner ytd-channel-name a, #upload-info ytd-channel-name a',
	);
	return nonEmpty(anchor?.href);
}

function readChannelName(): string | null {
	return (
		readMicroformatAttr('link[itemprop="name"]', 'content') ??
		nonEmpty(
			document.querySelector('#owner ytd-channel-name a, #upload-info ytd-channel-name a')
				?.textContent,
		)
	);
}

function readChannelHandle(): string | null {
	const url = readChannelUrl();
	if (!url) return null;
	const match = url.match(/\/@([^/?#]+)/);
	return match ? `@${match[1]}` : null;
}

function readPublishedAt(): string | null {
	return (
		readMicroformatAttr('meta[itemprop="datePublished"]', 'content') ??
		readMicroformatAttr('meta[itemprop="uploadDate"]', 'content')
	);
}

function readViewCount(): number | null {
	return parseDigits(readMicroformatAttr('meta[itemprop="interactionCount"]', 'content'));
}

function readLikeCount(): number | null {
	/// The like button's `aria-label` encodes the count, e.g. "like this
	/// video along with 12,345 other people". The DOM path changes
	/// often, so accept either the legacy or the `like-button-view-model`
	/// container.
	const btn = document.querySelector<HTMLElement>(
		'ytd-watch-metadata like-button-view-model button, ' +
			'ytd-watch-metadata #segmented-like-button button, ' +
			'#top-level-buttons-computed like-button-view-model button',
	);
	return parseDigits(btn?.getAttribute('aria-label') ?? null);
}

function readDescription(): string {
	const el =
		document.querySelector('#description-inline-expander') ??
		document.querySelector('#description #attributed-snippet-text') ??
		document.querySelector('ytd-watch-metadata #description');
	return nonEmpty(el?.textContent) ?? '';
}

export async function executeGetVideoMetadata(): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const clamped = clampString(readDescription(), READABILITY_BODY_CAP);
	return {
		video_id: videoId,
		title: readTitle(),
		channel_name: readChannelName(),
		channel_handle: readChannelHandle(),
		channel_url: readChannelUrl(),
		published_at: readPublishedAt(),
		view_count: readViewCount(),
		like_count: readLikeCount(),
		description: clamped.value,
		description_truncated: clamped.truncated,
	};
}

export const getVideoMetadata: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_get_video_metadata',
		description:
			"Return structured metadata for the active YouTube watch page: title, channel name / handle / URL, ISO publish date, view and like counts (best-effort), plus the visible video description. Fields that aren't surfaced in the DOM at call time come back as `null`; the description is truncated and `description_truncated` flags when that happened.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 2_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeGetVideoMetadata();
	},
};
