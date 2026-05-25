import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import { Readability } from '@mozilla/readability';
import { READABILITY_BODY_CAP, clampString } from '../../extensions/web/truncation';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	title: z.string().nullable(),
	byline: z.string().nullable(),
	site_name: z.string().nullable(),
	language: z.string().nullable(),
	excerpt: z.string().nullable(),
	content_html: z.string(),
	text_content: z.string(),
	length: z.number(),
});

type Result = z.infer<typeof Out>;

function nonEmpty(value: string | null | undefined): string | null {
	if (!value) return null;
	const trimmed = value.trim();
	return trimmed.length > 0 ? trimmed : null;
}

/// Run Mozilla Readability against a clone of the live document and
/// emit a `ReadabilityArticle`. Both `content_html` and `text_content`
/// are truncated to `READABILITY_BODY_CAP` bytes; `length` reports the
/// pre-truncation character count so the model can tell when content
/// was elided.
export async function executeGetReadabilityArticle(): Promise<Result> {
	const clone = document.cloneNode(true) as Document;
	const parsed = new Readability(clone).parse();
	const html = clampString(parsed?.content ?? '', READABILITY_BODY_CAP);
	const text = clampString(parsed?.textContent ?? '', READABILITY_BODY_CAP);
	return {
		title: nonEmpty(parsed?.title) ?? (document.title.trim() || null),
		byline: nonEmpty(parsed?.byline),
		site_name: nonEmpty(parsed?.siteName),
		language: nonEmpty(parsed?.lang),
		excerpt: nonEmpty(parsed?.excerpt),
		content_html: html.value,
		text_content: text.value,
		length: parsed?.length ?? text.value.length,
	};
}

export const getReadabilityArticle: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'web_get_readability_article',
		description:
			"Run Mozilla Readability against the active page and return the main article content as cleaned-up HTML plus plain text, along with title, byline, site name, language, and excerpt. Both bodies are truncated; `length` reports the pre-truncation character count.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 5_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return executeGetReadabilityArticle();
	},
};
