import type { WordDocumentAsset } from '$lib/shared/bindings';

export interface DocumentMetadata {
	title: string;
	author: string;
	last_modified: string | null;
	word_count: number;
}

export async function getDocumentAsset(): Promise<WordDocumentAsset> {
	return await Word.run(async (context) => {
		const body = context.document.body;
		const props = context.document.properties;
		body.load('text');
		props.load('title');
		await context.sync();
		const title = props.title?.trim();
		return {
			document_name: title !== undefined && title.length > 0 ? title : 'Untitled',
			text: body.text,
		};
	});
}

export async function getDocumentMetadata(): Promise<DocumentMetadata> {
	return await Word.run(async (context) => {
		const props = context.document.properties;
		const body = context.document.body;
		props.load(['title', 'author', 'lastModifiedBy']);
		// `Document.properties` does not expose lastModified directly; we
		// fall back to null so the desktop can decide what to do with it.
		body.load('text');
		await context.sync();

		const title = props.title?.trim();
		const author = props.author?.trim();
		const wordCount = countWords(body.text);

		return {
			title: title !== undefined && title.length > 0 ? title : 'Untitled',
			author: author !== undefined && author.length > 0 ? author : 'Unknown',
			last_modified: null,
			word_count: wordCount,
		};
	});
}

export function countWords(text: string): number {
	const trimmed = text.trim();
	if (trimmed.length === 0) return 0;
	return trimmed.split(/\s+/u).length;
}
