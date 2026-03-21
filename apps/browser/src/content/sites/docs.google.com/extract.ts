import browser from 'webextension-polyfill';
import type { NativeArticleAsset } from '../../../shared/content/bindings';
import type { NativeResponse } from '../../../shared/content/models';

function getDocTitle(): string {
	const titleInput = document.querySelector<HTMLInputElement>('.docs-title-input');
	if (titleInput?.value) return titleInput.value;
	const titleWidget = document.querySelector('.docs-title-widget');
	if (titleWidget?.textContent) return titleWidget.textContent.trim();
	return document.title.replace(/ - Google Docs$/, '');
}

function getDocumentId(): string | null {
	const match = window.location.pathname.match(/\/document\/d\/([a-zA-Z0-9_-]+)/);
	return match?.[1] ?? null;
}

// Text is error if ok is false
type FetchUrlResponse = { ok: true; text: string };

async function fetchDocumentText(docId: string): Promise<string> {
	const exportUrl = `https://docs.google.com/document/d/${docId}/export?format=txt`;
	const result = (await browser.runtime.sendMessage({
		type: 'FETCH_URL',
		url: exportUrl,
	})) as FetchUrlResponse;
	if (!result.ok) {
		throw new Error(result.text);
	}
	return result.text;
}

export async function createGoogleDocsAsset(): Promise<NativeResponse> {
	try {
		const title = getDocTitle();
		const docId = getDocumentId();
		if (!docId) {
			return { kind: 'Error', data: 'Could not extract document ID from URL' };
		}

		const textContent = await fetchDocumentText(docId);

		const reportData: NativeArticleAsset = {
			title,
			url: window.location.href,
			content: '',
			text_content: textContent,
			site_name: 'Google Docs',
			selected_text: window.getSelection()?.toString() || '',
			language: document.documentElement.lang || '',
			excerpt: textContent.slice(0, 200),
			length: textContent.length,
		};

		return { kind: 'NativeArticleAsset', data: reportData };
	} catch (error) {
		const errorMessage = error instanceof Error ? error.message : String(error);
		console.error('Error extracting Google Docs content:', {
			url: window.location.href,
			error: errorMessage,
		});
		return { kind: 'Error', data: `Failed to extract Google Docs content: ${errorMessage}` };
	}
}
