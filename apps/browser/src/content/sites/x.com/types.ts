import type { MessageType, BrowserObj } from '../../../shared/content/extensions/watchers/watcher';

export type TwitterMessageType = MessageType;

export interface WatcherParams {
	currentUrl?: string;
	pageTitle?: string;
}

export interface TwitterBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: TwitterMessageType;
}
