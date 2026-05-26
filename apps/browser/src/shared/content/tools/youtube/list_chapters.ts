import { parseHmsTimestamp, readPlayerTime, requireCurrentVideoId } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Chapter = z.object({
	start: z.number().nonnegative(),
	end: z.number().nonnegative().nullable(),
	label: z.string(),
});

const Out = z.object({
	video_id: z.string(),
	chapters: z.array(Chapter),
});

type Result = z.infer<typeof Out>;
type ChapterT = z.infer<typeof Chapter>;

/// Scrape chapter rows from `ytd-macro-markers-list-renderer` — YouTube
/// only renders this element when the video actually has chapters, so
/// an empty result is the right signal for "no chapters" rather than
/// an error.
function scrapeChapters(): ChapterT[] {
	const items = document.querySelectorAll<HTMLElement>(
		'ytd-macro-markers-list-renderer ytd-macro-markers-list-item-renderer',
	);
	const chapters: ChapterT[] = [];
	for (const item of Array.from(items)) {
		const timeText = item.querySelector('#time')?.textContent?.trim();
		const labelText = item.querySelector('h4')?.textContent?.trim();
		if (!timeText || !labelText) continue;
		const start = parseHmsTimestamp(timeText);
		if (start === null) continue;
		chapters.push({ start, end: null, label: labelText });
	}
	return chapters;
}

/// Fill in each chapter's `end` from the next chapter's `start`; the
/// final chapter's `end` is the player duration when readable, otherwise
/// left as `null` so the model can tell the boundary wasn't observed.
function fillEnds(chapters: ChapterT[]): ChapterT[] {
	if (chapters.length === 0) return chapters;
	const filled: ChapterT[] = chapters.map((chapter, i) => {
		const next = chapters[i + 1];
		return next ? { ...chapter, end: next.start } : chapter;
	});
	const lastIndex = filled.length - 1;
	if (filled[lastIndex].end === null) {
		const duration = readPlayerTime()?.duration ?? null;
		if (duration !== null) {
			filled[lastIndex] = { ...filled[lastIndex], end: duration };
		}
	}
	return filled;
}

export async function executeListChapters(): Promise<Result> {
	return {
		video_id: requireCurrentVideoId(),
		chapters: fillEnds(scrapeChapters()),
	};
}

export const listChapters: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_list_chapters',
		description:
			"Return the active YouTube video's chapter list — each entry has `start` (seconds), `end` (seconds, derived from the next chapter; the final chapter's end falls back to the player duration), and the chapter `label`. Returns an empty array (no error) when the video has no chapters.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 2_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeListChapters();
	},
};
