import {
	Watcher,
	type WatcherResponse,
} from '@eurora/browser-shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import {
	createArticleAsset,
	createArticleSnapshot,
} from '@eurora/browser-shared/content/extensions/article/util';
import { PdfBrowserMessage, type WatcherParams } from './types.js';
import type { NativePdfAsset } from '@eurora/browser-shared/content/bindings';

export class PdfWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public listen(
		obj: PdfBrowserMessage,
		sender: browser.Runtime.MessageSender,
		response: (response?: WatcherResponse) => void,
	): boolean {
		const { type } = obj;

		let promise: Promise<WatcherResponse>;

		switch (type) {
			case 'NEW':
				promise = this.handleNew(obj, sender);
				break;
			case 'GENERATE_ASSETS':
				promise = this.handleGenerateAssets(obj, sender);
				break;
			case 'GENERATE_SNAPSHOT':
				promise = this.handleGenerateSnapshot(obj, sender);
				break;
			default:
				response({ kind: 'Error', data: 'Invalid message type' });
				return false;
		}

		promise
			.then((result) => {
				response(result);
			})
			.catch((error) => {
				const message = error instanceof Error ? error.message : String(error);
				console.error('Pdf watcher failed', { error });
				response({ kind: 'Error', data: message });
			});

		return true;
	}

	public async handleNew(
		_obj: PdfBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		console.log('PDF Watcher: New PDF detected');
	}

	public async handleGenerateAssets(
		_obj: PdfBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		const pdfViewerApplication = globalThis['PDFViewerApplication'];
		if (!pdfViewerApplication) {
			return { kind: 'Error', data: 'PDFViewerApplication not found' };
		}
		const content = await getPageContent(pdfViewerApplication);
		return {
			kind: 'NativePdfAsset',
			data: {
				url: pdfViewerApplication.url,
				content,
				title: document.title,
				selected_text: window.getSelection().toString() ?? null,
			} as NativePdfAsset,
		};
	}

	public async handleGenerateSnapshot(
		_obj: PdfBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		// return createArticleSnapshot(window);
	}
}

export function main() {
	const watcher = new PdfWatcher({});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}

async function getPageContent(application: any): Promise<string> {
	const pdfDoc = application.pdfViewer.pdfDocument;
	const currentPage = application.pdfViewer.currentPageNumber;

	const page = await pdfDoc.getPage(currentPage);
	const content = await page.getTextContent();

	return content.items.map((item) => item.str).join(' ');
}
