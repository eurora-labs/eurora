import {
	TranscriptArgs,
	fetchTranscriptOrExplain,
	filterTranscriptRange,
	requireCurrentVideoId,
	type TranscriptArgsT,
} from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Entry = z.object({
	start: z.number().nonnegative(),
	duration: z.number().nonnegative(),
	text: z.string(),
});

const Out = z.object({
	video_id: z.string(),
	language: z.string(),
	is_generated: z.boolean(),
	entries: z.array(Entry),
});

type Result = z.infer<typeof Out>;

function normalizeText(input: string): string {
	return input.replace(/\s+/g, ' ').trim();
}

export async function executeGetTimedTranscript(args: TranscriptArgsT): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const fetched = await fetchTranscriptOrExplain(videoId, args.language);
	const included = filterTranscriptRange(fetched.snippets, args.start, args.end);
	return {
		video_id: fetched.videoId,
		language: fetched.languageCode,
		is_generated: fetched.isGenerated,
		entries: included.map((s) => ({
			start: s.start,
			duration: s.duration,
			text: normalizeText(s.text),
		})),
	};
}

export const getTimedTranscript: Tool<typeof TranscriptArgs, Result> = {
	descriptor: {
		name: 'youtube_get_timed_transcript',
		description:
			"Return the active YouTube video's transcript as time-stamped entries. Each entry carries `start` (seconds from the start of the video), `duration` (seconds), and the line `text`. Prefer `youtube_get_transcript` when only the words matter — this one is the right call when you need to know **when** something was said, e.g. to construct a `youtube_seek_to` target or align quotes with playback position. Optional `start` / `end` seconds bound the returned entries to a sub-window; entries that overlap the window are included whole. Optional `language` is a YouTube caption-track code such as `'en'`, `'es'`, or `'pt-BR'` — call `youtube_list_captions` first to enumerate the codes available on this specific video (codes are short like `'en'`, NOT the spelled-out name `'english'`). Defaults to `'en'` when omitted; when the requested language has no track, the error message names the codes that ARE available. `language` (in the output) reports the source language code; `is_generated` is `true` for ASR tracks (their per-line timings drift by a beat — manual tracks are crisp). Fails when the video has no captions, is age- or region-restricted, or the YouTube data layer can't be reached.",
		parameters: zodToJsonSchema(TranscriptArgs) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 10_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: TranscriptArgs,
	async run(args) {
		return await executeGetTimedTranscript(args);
	},
};
