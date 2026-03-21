import type { MessageType, BrowserObj } from '../../../shared/content/extensions/watchers/watcher';

export type GoogleDocsMessageType = MessageType;

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface WatcherParams {}

export interface GoogleDocsBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: GoogleDocsMessageType;
}
