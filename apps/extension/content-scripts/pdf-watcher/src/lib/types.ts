import { MessageType, BrowserObj } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';

export type PdfMessageType = MessageType;

export interface WatcherParams {
	// TODO: Convert to PDFViewerApplication type from pdfjs lib
	pdfViewerApplication?: any;
}

export interface PdfChromeMessage extends Omit<BrowserObj, 'type'> {
	type: PdfMessageType;
}
