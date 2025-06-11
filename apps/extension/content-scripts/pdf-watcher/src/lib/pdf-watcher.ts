import { Watcher } from '@eurora/chrome-ext-shared/extensions/watchers/watcher.js';
import { PdfChromeMessage, type PdfMessageType, type WatcherParams } from './types.js';
import { ProtoPdfState } from '@eurora/shared/proto/tauri_ipc_pb.js';

interface PdfState extends Partial<ProtoPdfState> {
	type: 'PDF_STATE';
}

class PdfWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public listen(
		obj: PdfChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const { type } = obj;

		switch (type) {
			case 'NEW':
				this.handleNew(obj, sender, response);
				break;
			case 'GENERATE_ASSETS':
				this.handleGenerateAssets(obj, sender, response);
				break;
			case 'GENERATE_SNAPSHOT':
				this.handleGenerateSnapshot(obj, sender, response);
				break;
			default:
				response();
		}
	}

	public handleNew(
		obj: PdfChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		console.log('PDF Watcher: New PDF detected');
		// Initialize PDF viewer application reference
		this.params.pdfViewerApplication = globalThis['PDFViewerApplication'];
		response();
	}

	public handleGenerateAssets(
		obj: PdfChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		console.log('Generating PDF report for URL:', window.location.href);

		this.getPdfState()
			.then((pdfState) => {
				response(pdfState);
			})
			.catch((error) => {
				const errorMessage = error instanceof Error ? error.message : String(error);
				const contextualError = `Failed to generate PDF assets for ${window.location.href}: ${errorMessage}`;
				console.error('Error generating PDF report:', {
					url: window.location.href,
					error: errorMessage,
					stack: error instanceof Error ? error.stack : undefined,
				});
				response({
					success: false,
					error: contextualError,
					context: {
						url: window.location.href,
						timestamp: new Date().toISOString(),
					},
				});
			});

		return true; // Important: indicates we'll send response asynchronously
	}

	public handleGenerateSnapshot(
		obj: PdfChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		console.log('Generating PDF snapshot');
		// For PDF, snapshot is the same as assets for now
		this.handleGenerateAssets(obj, sender, response);
		return true;
	}

	private async getPdfState(): Promise<PdfState> {
		const pdfViewerApplication =
			this.params.pdfViewerApplication || globalThis['PDFViewerApplication'];
		if (!pdfViewerApplication) {
			throw new Error('PDFViewerApplication not found');
		}

		const content = await this.getPageContent(pdfViewerApplication);

		return {
			type: 'PDF_STATE',
			url: window.location.href,
			title: document.title,
			content,
			selectedText: window.getSelection()?.toString() ?? '',
		};
	}

	private async getPageContent(application: any): Promise<string> {
		const pdfDoc = application.pdfViewer.pdfDocument;
		const currentPage = application.pdfViewer.currentPageNumber;

		const page = await pdfDoc.getPage(currentPage);
		const content = await page.getTextContent();

		return content.items.map((item: any) => item.str).join(' ');
	}
}

(() => {
	console.log('Eurora v5 PDF Watcher content script loaded');

	const watcher = new PdfWatcher({
		pdfViewerApplication: globalThis['PDFViewerApplication'],
	});

	chrome.runtime.onMessage.addListener(watcher.listen.bind(watcher));
})();
