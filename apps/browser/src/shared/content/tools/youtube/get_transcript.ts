import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import { YouTubeTranscriptApi } from '../../../../content/sites/youtube.com/transcript';
import { requireCurrentVideoId } from './_lib';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const TranscriptEntry = z.object({
	start: z.number(),
	duration: z.number(),
	text: z.string(),
});

const Out = z.object({
	video_id: z.string(),
	language: z.string(),
	entries: z.array(TranscriptEntry),
});

type Result = z.infer<typeof Out>;

/// Singleton — the API instance carries no per-call state, so reusing
/// one across tool calls avoids re-spinning up its internal caches on
/// every transcript fetch.
const TRANSCRIPT_API = new YouTubeTranscriptApi();

export async function executeGetTranscript(): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const fetched = await TRANSCRIPT_API.fetch(videoId);
	return {
		video_id: fetched.videoId,
		language: fetched.languageCode,
		entries: fetched.snippets.map((s) => ({
			start: s.start,
			duration: s.duration,
			text: s.text,
		})),
	};
}

export const getTranscript: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_get_transcript',
		description:
			"Return the active YouTube video's transcript as time-stamped entries (start, duration, text) along with the source language code. Auto-generated tracks report the ASR language; manual tracks report the author-specified locale. Fails when the video has no captions, the user is age- or region-restricted, or the YouTube data layer can't be reached.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 10_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return executeGetTranscript();
	},
};
