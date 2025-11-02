import { Readability } from '@mozilla/readability';
import type { NativeArticleAsset, NativeArticleSnapshot } from '../../bindings.js';
import type { NativeResponse } from '../../models.js';

export function createArticleAsset(document: Document): NativeResponse {
	try {
		const clone = document.cloneNode(true) as Document;
		const article = new Readability(clone).parse();

		const reportData: NativeArticleAsset = {
			content: article?.content || '',
			text_content: article?.textContent || '',
			title: article?.title || document.title,
			site_name: article?.siteName || '',
			language: article?.lang || '',
			excerpt: article?.excerpt || '',
			length: article?.length || 0,
			selected_text: window.getSelection()?.toString() || '',
			url: window.location.href,
		};

		return { kind: 'NativeArticleAsset', data: reportData };
	} catch (error) {
		const errorMessage = error instanceof Error ? error.message : String(error);
		const contextualError = `Failed to generate article assets for ${window.location.href}: ${errorMessage}`;

		console.error('Error generating article report:', {
			url: window.location.href,
			error: errorMessage,
			stack: error instanceof Error ? error.stack : undefined,
		});

		return { kind: 'Error', data: contextualError };
	}
}

export function createArticleSnapshot(window: Window): NativeResponse {
	const selectedText = window.getSelection()?.toString() || '';
	const snapshot: NativeArticleSnapshot = {
		highlighted_text: selectedText,
	};

	return { kind: 'NativeArticleSnapshot', data: snapshot };
}
