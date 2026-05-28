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

const Out = z.object({
	video_id: z.string(),
	language: z.string(),
	is_generated: z.boolean(),
	text: z.string(),
});

type Result = z.infer<typeof Out>;

function normalizeText(input: string): string {
	return input.replace(/\s+/g, ' ').trim();
}

export async function executeGetTranscript(args: TranscriptArgsT): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const fetched = await fetchTranscriptOrExplain(videoId, args.language);
	const included = filterTranscriptRange(fetched.snippets, args.start, args.end);
	return {
		video_id: fetched.videoId,
		language: fetched.languageCode,
		is_generated: fetched.isGenerated,
		text: normalizeText(included.map((s) => s.text).join(' ')),
	};
}

export const getTranscript: Tool<typeof TranscriptArgs, Result> = {
	descriptor: {
		name: 'youtube_get_transcript',
		description:
			"Return the active YouTube video's transcript as plain text — entry timestamps are not surfaced. Call `youtube_get_timed_transcript` instead when you need per-line timing (e.g. to construct a `youtube_seek_to` target). Optional `start` / `end` seconds bound the returned text to a sub-window; entries that overlap the window are included whole. Optional `language` is a YouTube caption-track code such as `'en'`, `'es'`, or `'pt-BR'` — call `youtube_list_captions` first to enumerate the codes available on this specific video (codes are short like `'en'`, NOT the spelled-out name `'english'`). Defaults to `'en'` when omitted; when the requested language has no track, the error message names the codes that ARE available. `language` (in the output) reports the source language code; `is_generated` is `true` for ASR tracks (manual tracks report the author-specified locale). Fails when the video has no captions, is age- or region-restricted, or the YouTube data layer can't be reached.",
		parameters: zodToJsonSchema(TranscriptArgs) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 10_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: TranscriptArgs,
	async run(args) {
		return await executeGetTranscript(args);
	},
};
