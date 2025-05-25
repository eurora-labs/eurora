import { ProtoPdfState } from '@eurora/proto/tauri_ipc';

interface PdfState extends Partial<ProtoPdfState> {
	type: 'PDF_STATE';
}

(() => {
	console.log('Eurora v5 PDF Watcher content script loaded');

	let pdfViewerApplication = globalThis['PDFViewerApplication'];

	chrome.runtime.onMessage.addListener((obj, sender, response) => {
		const { type } = obj;

		switch (type) {
			case 'GENERATE_ASSETS':
				console.log('Generating PDF report for URL:', window.location.href);

				getPdfState()
					.then((pdfState) => {
						response(pdfState);
					})
					.catch((error) => {
						const errorMessage = error instanceof Error ? error.message : String(error);
						const contextualError = `Failed to generate PDF assets for ${window.location.href}: ${errorMessage}`;
						console.error('Error generating PDF report:', {
							url: window.location.href,
							error: errorMessage,
							stack: error instanceof Error ? error.stack : undefined
						});
						response({
							success: false,
							error: contextualError,
							context: {
								url: window.location.href,
								timestamp: new Date().toISOString()
							}
						});
					});

				return true;
			default:
				response();
		}
	});

	async function getPdfState(): Promise<PdfState> {
		pdfViewerApplication = globalThis['PDFViewerApplication'];
		if (!pdfViewerApplication) {
			throw new Error('PDFViewerApplication not found');
		}

		const content = await getPageContent(pdfViewerApplication);

		return {
			type: 'PDF_STATE',
			url: window.location.href,
			title: document.title,
			content,
			selectedText: window.getSelection().toString() ?? ''
		};
	}

	async function getPageContent(application: any): Promise<string> {
		const pdfDoc = application.pdfViewer.pdfDocument;
		const currentPage = application.pdfViewer.currentPageNumber;

		const page = await pdfDoc.getPage(currentPage);
		const content = await page.getTextContent();

		return content.items.map((item) => item.str).join(' ');
	}
})();
