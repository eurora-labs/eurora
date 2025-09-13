import { MessageType, ChromeObj } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { NativeTwitterTweet } from '@eurora/chrome-ext-shared/bindings';

type CustomMessageType = 'TEST';
export type TwitterMessageType = MessageType | CustomMessageType;

export interface WatcherParams {
	currentUrl?: string;
	pageTitle?: string;
	tweets: NativeTwitterTweet[];
}

export interface TwitterChromeMessage extends Omit<ChromeObj, 'type'> {
	type: TwitterMessageType;
}
