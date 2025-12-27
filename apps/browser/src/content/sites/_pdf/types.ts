import type {
	MessageType,
	BrowserObj,
} from '@eurora/browser-shared/content/extensions/watchers/watcher';

export type PdfMessageType = MessageType;

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface WatcherParams {}

export interface PdfBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: PdfMessageType;
}
