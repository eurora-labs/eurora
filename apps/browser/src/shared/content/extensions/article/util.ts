import { Readability } from '@mozilla/readability';
import type { NativeArticleAsset, NativeArticleSnapshot } from '../../bindings';
import type { NativeResponse } from '../../models';

export function createArticleAsset(document: Document): NativeResponse {
	try {
		const clone = document.cloneNode(true) as Document;
		const article = new Readability(clone).parse();

		const reportData: NativeArticleAsset = {
			title: article?.title || document.title,
			url: window.location.href,
			content: article?.content || '',
			text_content: article?.textContent || '',
			site_name: article?.siteName || '',
			selected_text: window.getSelection()?.toString() || '',
			language: article?.lang || '',
			excerpt: article?.excerpt || '',
			length: article?.length || 0,
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
