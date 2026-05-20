import { READABILITY_BODY_CAP, clampString } from './truncation';
import { Readability } from '@mozilla/readability';
import type { ReadabilityArticle } from '../../bindings';

/**
 * Run Mozilla Readability against a clone of the live document and emit
 * a [`ReadabilityArticle`]. Both `content_html` and `text_content` are
 * truncated to [`READABILITY_BODY_CAP`] bytes; `length` reports the
 * pre-truncation character count so the model can tell when content was
 * elided.
 *
 * Returns the article data structure directly. The bridge layer
 * forwards the content-script reply verbatim as the response payload;
 * the desktop dispatcher then deserialises that payload into
 * [`ReadabilityArticle`] (Rust side). Wrapping the value in a
 * `{kind, data}` envelope here would break that decode — the YouTube
 * tools (which work) return their data structures directly for the
 * same reason.
 */
export async function handleGetReadabilityArticle(): Promise<ReadabilityArticle> {
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

function nonEmpty(value: string | null | undefined): string | null {
	if (!value) {
		return null;
	}
	const trimmed = value.trim();
	return trimmed.length > 0 ? trimmed : null;
}
