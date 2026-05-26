import { requireCurrentVideoId } from './_lib';
import { YouTubeTranscriptApi } from '../../../../content/sites/youtube.com/transcript';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z
	.object({
		start: z.number().nonnegative().optional(),
		end: z.number().nonnegative().optional(),
	})
	.strict()
	.refine((a) => a.start === undefined || a.end === undefined || a.end > a.start, {
		message: '`end` must be greater than `start`',
	});

const Out = z.object({
	video_id: z.string(),
	language: z.string(),
	is_generated: z.boolean(),
	text: z.string(),
});

type Result = z.infer<typeof Out>;

/// Singleton — the API instance carries no per-call state, so reusing
/// one across tool calls avoids re-spinning up its internal caches on
/// every transcript fetch.
const TRANSCRIPT_API = new YouTubeTranscriptApi();

function normalizeText(input: string): string {
	return input.replace(/\s+/g, ' ').trim();
}

export async function executeGetTranscript(args: z.infer<typeof Args>): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const fetched = await TRANSCRIPT_API.fetch(videoId);

	const lowerBound = args.start ?? 0;
	const upperBound = args.end ?? Number.POSITIVE_INFINITY;
	/// Overlap-include rather than strict-contain: a snippet that spans
	/// the boundary is kept whole instead of being silently dropped, so
	/// the returned text doesn't lose the partial entry at each edge.
	const included = fetched.snippets.filter(
		(s) => s.start + s.duration > lowerBound && s.start < upperBound,
	);

	return {
		video_id: fetched.videoId,
		language: fetched.languageCode,
		is_generated: fetched.isGenerated,
		text: normalizeText(included.map((s) => s.text).join(' ')),
	};
}

export const getTranscript: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_get_transcript',
		description:
			"Return the active YouTube video's transcript as plain text — entry timestamps are not surfaced. Optional `start` / `end` seconds bound the returned text to a sub-window; transcript entries that overlap the window are included whole. `language` reports the source language code (auto-generated tracks report the ASR language; manual tracks report the author-specified locale); `is_generated` is `true` for ASR tracks. Fails when the video has no captions, is age- or region-restricted, or the YouTube data layer can't be reached.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 10_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return await executeGetTranscript(args);
	},
};
