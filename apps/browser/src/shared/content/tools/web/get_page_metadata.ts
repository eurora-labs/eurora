import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

/// Empty arg schema. `.strict()` rejects unknown keys so the LLM can't
/// smuggle in fields the descriptor doesn't advertise.
const Args = z.object({}).strict();

const Viewport = z.object({
	scroll_x: z.number().nullable(),
	scroll_y: z.number().nullable(),
	inner_width: z.number().nullable(),
	inner_height: z.number().nullable(),
	document_height: z.number().nullable(),
});

const Out = z.object({
	url: z.string(),
	title: z.string(),
	host: z.string(),
	language: z.string().nullable(),
	charset: z.string().nullable(),
	description: z.string().nullable(),
	og: z.record(z.string()),
	viewport: Viewport,
});

type Result = z.infer<typeof Out>;

function safeParseUrl(href: string): URL | null {
	try {
		return new URL(href);
	} catch {
		return null;
	}
}

function resolveLanguage(): string | null {
	const htmlLang = document.documentElement.lang?.trim();
	if (htmlLang) return htmlLang;
	const httpEquiv = document.querySelector<HTMLMetaElement>(
		'meta[http-equiv="content-language" i]',
	);
	const content = httpEquiv?.content?.trim();
	return content ? content : null;
}

function readMetaContent(attr: 'name' | 'property', key: string): string | null {
	const escaped = key.replace(/"/g, '\\"');
	const meta = document.querySelector<HTMLMetaElement>(`meta[${attr}="${escaped}" i]`);
	const value = meta?.content?.trim();
	return value ? value : null;
}

function readOpenGraph(): Record<string, string> {
	const result: Record<string, string> = {};
	const tags = document.querySelectorAll<HTMLMetaElement>('meta[property^="og:" i]');
	for (const tag of Array.from(tags)) {
		const property = tag.getAttribute('property');
		if (!property) continue;
		const key = property.slice(3).trim();
		if (key.length === 0) continue;
		if (key in result) continue;
		const value = tag.content?.trim();
		if (value) result[key] = value;
	}
	return result;
}

function numericOrNull(value: number | null): number | null {
	if (value === null) return null;
	return Number.isFinite(value) ? value : null;
}

function readViewport(): z.infer<typeof Viewport> {
	const doc = document.documentElement;
	return {
		scroll_x: numericOrNull(window.scrollX),
		scroll_y: numericOrNull(window.scrollY),
		inner_width: numericOrNull(window.innerWidth),
		inner_height: numericOrNull(window.innerHeight),
		document_height: numericOrNull(doc?.scrollHeight ?? null),
	};
}

/// Resolve page-level metadata for the active document. Language
/// resolution: `<html lang>` → `<meta http-equiv="content-language">` →
/// `null`. OpenGraph tags are scraped into a flat map keyed by the
/// suffix after `og:`; duplicates keep the document-order winner.
export async function executeGetPageMetadata(): Promise<Result> {
	const url = window.location.href;
	const parsed = safeParseUrl(url);
	return {
		url,
		title: document.title,
		host: parsed?.host ?? '',
		language: resolveLanguage(),
		charset: document.characterSet || null,
		description: readMetaContent('name', 'description'),
		og: readOpenGraph(),
		viewport: readViewport(),
	};
}

export const getPageMetadata: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'web_get_page_metadata',
		description:
			'Return page-level metadata for the active tab: URL, title, host, language, charset, description, OpenGraph tags, and viewport metrics. The model uses this to ground itself in what the user is looking at without needing the full page.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 1_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeGetPageMetadata();
	},
};
