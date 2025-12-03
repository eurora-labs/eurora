import {
	MessageType,
	BrowserObj,
} from '@eurora/browser-shared/content/extensions/watchers/watcher';
import { NativeTwitterTweet } from '@eurora/browser-shared/content/bindings';

export type TwitterMessageType = MessageType;

export interface WatcherParams {
	currentUrl?: string;
	pageTitle?: string;
	tweets: NativeTwitterTweet[];
}

export interface TwitterBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: TwitterMessageType;
}
