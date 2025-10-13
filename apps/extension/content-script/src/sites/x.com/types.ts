import { MessageType, BrowserObj } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { NativeTwitterTweet } from '@eurora/chrome-ext-shared/bindings';

export type TwitterMessageType = MessageType;

export interface WatcherParams {
	currentUrl?: string;
	pageTitle?: string;
	tweets: NativeTwitterTweet[];
}

export interface TwitterChromeMessage extends Omit<BrowserObj, 'type'> {
	type: TwitterMessageType;
}
