import {
	MessageType,
	BrowserObj,
} from '@eurora/browser-shared/content/extensions/watchers/watcher';

export type PdfMessageType = MessageType;

export interface WatcherParams {}

export interface YoutubeBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: PdfMessageType;
}
