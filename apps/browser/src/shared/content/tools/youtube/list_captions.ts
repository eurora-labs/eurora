import { TRANSCRIPT_API, requireCurrentVideoId } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const TrackEntry = z.object({
	language: z.string(),
	language_code: z.string(),
	is_translatable: z.boolean(),
});

const TranslationLanguage = z.object({
	language: z.string(),
	language_code: z.string(),
});

const Out = z.object({
	video_id: z.string(),
	manually_created: z.array(TrackEntry),
	generated: z.array(TrackEntry),
	translation_languages: z.array(TranslationLanguage),
});

type Result = z.infer<typeof Out>;
type TrackEntryT = z.infer<typeof TrackEntry>;
type TranslationLanguageT = z.infer<typeof TranslationLanguage>;

export async function executeListCaptions(): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const list = await TRANSCRIPT_API.list(videoId);

	const manuallyCreated: TrackEntryT[] = [];
	const generated: TrackEntryT[] = [];
	/// Dedupe translation languages by code: YouTube attaches the same
	/// translation list to every translatable track, so iterating the
	/// tracks would otherwise emit each language once per track.
	const translations = new Map<string, string>();

	for (const transcript of list) {
		const entry: TrackEntryT = {
			language: transcript.language,
			language_code: transcript.languageCode,
			is_translatable: transcript.isTranslatable,
		};
		if (transcript.isGenerated) {
			generated.push(entry);
		} else {
			manuallyCreated.push(entry);
		}
		for (const tl of transcript.translationLanguages) {
			translations.set(tl.languageCode, tl.language);
		}
	}

	const translation_languages: TranslationLanguageT[] = Array.from(
		translations,
		([language_code, language]) => ({ language, language_code }),
	);

	return {
		video_id: videoId,
		manually_created: manuallyCreated,
		generated,
		translation_languages,
	};
}

export const listCaptions: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_list_captions',
		description:
			"Enumerate the active video's caption tracks — manually-created and auto-generated (ASR) — along with the translation languages available for translatable tracks. Each entry carries the human-readable `language`, the BCP47-ish `language_code` YouTube uses, and `is_translatable`. Fails when the video has no captions, is age- or region-restricted, or the YouTube data layer can't be reached.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 10_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeListCaptions();
	},
};
