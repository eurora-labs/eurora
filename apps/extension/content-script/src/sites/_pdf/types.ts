import {
	MessageType,
	BrowserObj,
} from '@eurora/browser-shared/content/extensions/watchers/watcher';

export type PdfMessageType = MessageType;

export interface WatcherParams {}

export interface PdfBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: PdfMessageType;
}
