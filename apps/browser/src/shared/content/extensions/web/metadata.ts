import type { PageMetadata, ViewportMetrics } from '../../bindings';

/**
 * Resolve [`PageMetadata`] for the active document.
 *
 * Language resolution order, matching the Rust-side doc comment on
 * [`PageMetadata::language`]:
 *
 *   1. `document.documentElement.lang`,
 *   2. `<meta http-equiv="content-language">` `content`,
 *   3. `null`.
 *
 * OpenGraph tags are scraped from `<meta property="og:*">` into a flat
 * map keyed by the suffix after `og:`. Duplicate properties keep the
 * first value encountered (document order).
 *
 * Returns the metadata structure directly; see the note on
 * `handleGetReadabilityArticle` for why the response is not wrapped in
 * a `{kind, data}` envelope.
 */
export async function handleGetPageMetadata(): Promise<PageMetadata> {
	const url = window.location.href;
	const parsed = safeParseUrl(url);
	const title = document.title;
	const host = parsed?.host ?? '';
	const language = resolveLanguage();
	const charset = document.characterSet || null;
	const description = readMetaContent('name', 'description');
	const og = readOpenGraph();
	const viewport = readViewport();

	return {
		url,
		title,
		host,
		language,
		charset,
		description,
		og,
		viewport,
	};
}

function safeParseUrl(href: string): URL | null {
	try {
		return new URL(href);
	} catch {
		return null;
	}
}

function resolveLanguage(): string | null {
	const htmlLang = document.documentElement.lang?.trim();
	if (htmlLang) {
		return htmlLang;
	}
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
		if (!property) {
			continue;
		}
		// Strip the `og:` prefix case-insensitively without using regex
		// capture groups (avoids dragging a runtime dependency on the
		// `/og:(.*)/i` shape into a hot path).
		const key = property.slice(3).trim();
		if (key.length === 0) {
			continue;
		}
		if (key in result) {
			// Document-order wins; duplicates are dropped on the floor.
			continue;
		}
		const value = tag.content?.trim();
		if (value) {
			result[key] = value;
		}
	}
	return result;
}

function readViewport(): ViewportMetrics {
	const doc = document.documentElement;
	return {
		scroll_x: numericOrNull(window.scrollX),
		scroll_y: numericOrNull(window.scrollY),
		inner_width: numericOrNull(window.innerWidth),
		inner_height: numericOrNull(window.innerHeight),
		document_height: numericOrNull(doc?.scrollHeight ?? null),
	};
}

function numericOrNull(value: number | null): number | null {
	if (value === null) {
		return null;
	}
	return Number.isFinite(value) ? value : null;
}
