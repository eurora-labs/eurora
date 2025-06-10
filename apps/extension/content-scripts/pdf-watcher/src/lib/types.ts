import { MessageType, ChromeObj } from '@eurora/chrome-ext-shared/extensions/watchers/watcher.js';

export type PdfMessageType = MessageType;

export interface WatcherParams {
	pdfViewerApplication?: any;
}

export interface PdfChromeMessage extends Omit<ChromeObj, 'type'> {
	type: PdfMessageType;
}
