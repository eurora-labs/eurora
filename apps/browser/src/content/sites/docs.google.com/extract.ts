import browser from 'webextension-polyfill';
import type { NativeArticleAsset } from '../../../shared/content/bindings';
import type { NativeResponse } from '../../../shared/content/models';

// Text is error if ok is false
type FetchUrlResponse = { ok: true; text: string };

type GoogleDocType = 'document' | 'spreadsheets';

function detectDocType(): GoogleDocType | null {
	const path = window.location.pathname;
	if (path.startsWith('/document/')) return 'document';
	if (path.startsWith('/spreadsheets/')) return 'spreadsheets';
	return null;
}

function getDocTitle(): string {
	const titleInput = document.querySelector<HTMLInputElement>('.docs-title-input');
	if (titleInput?.value) return titleInput.value;
	const titleWidget = document.querySelector('.docs-title-widget');
	if (titleWidget?.textContent) return titleWidget.textContent.trim();
	return document.title.replace(/ - Google (Docs|Sheets)$/, '');
}

function getResourceId(docType: GoogleDocType): string | null {
	const pattern = new RegExp(`\\/${docType}\\/d\\/([a-zA-Z0-9_-]+)`);
	const match = window.location.pathname.match(pattern);
	return match?.[1] ?? null;
}

async function fetchExport(docType: GoogleDocType, resourceId: string): Promise<string> {
	const format = docType === 'spreadsheets' ? 'csv' : 'txt';
	const exportUrl = `https://docs.google.com/${docType}/d/${resourceId}/export?format=${format}`;
	const result = (await browser.runtime.sendMessage({
		type: 'FETCH_URL',
		url: exportUrl,
	})) as FetchUrlResponse;
	if (!result.ok) {
		throw new Error(result.text);
	}
	return result.text;
}

function siteName(docType: GoogleDocType): string {
	return docType === 'spreadsheets' ? 'Google Sheets' : 'Google Docs';
}

export async function createGoogleDocsAsset(): Promise<NativeResponse> {
	try {
		const docType = detectDocType();
		if (!docType) {
			return { kind: 'Error', data: 'Unsupported Google Docs page type' };
		}

		const title = getDocTitle();
		const resourceId = getResourceId(docType);
		if (!resourceId) {
			return { kind: 'Error', data: 'Could not extract document ID from URL' };
		}

		const textContent = await fetchExport(docType, resourceId);

		const reportData: NativeArticleAsset = {
			title,
			url: window.location.href,
			content: '',
			text_content: textContent,
			site_name: siteName(docType),
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
