import type { NativeTwitterTweet } from '../../../shared/content/bindings';
import type { MessageType, BrowserObj } from '../../../shared/content/extensions/watchers/watcher';

export type TwitterMessageType = MessageType;

export interface WatcherParams {
	currentUrl?: string;
	pageTitle?: string;
	tweets: NativeTwitterTweet[];
}

export interface TwitterBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: TwitterMessageType;
}
