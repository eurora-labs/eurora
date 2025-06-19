import { MessageType, ChromeObj } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';

type CustomMessageType = 'TEST';
export type TwitterMessageType = MessageType | CustomMessageType;

export interface TwitterTweet {
	text: string;
	timestamp?: string;
	author?: string;
}

export interface WatcherParams {
	currentUrl?: string;
	pageTitle?: string;
	tweets: TwitterTweet[];
}

export interface TwitterChromeMessage extends Omit<ChromeObj, 'type'> {
	type: TwitterMessageType;
}
