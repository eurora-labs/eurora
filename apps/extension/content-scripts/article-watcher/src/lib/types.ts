import { MessageType, ChromeObj } from '@eurora/chrome-ext-shared/extensions/watchers/watcher.js';

export type ArticleMessageType = MessageType;

export interface WatcherParams {
	// Article watcher doesn't need specific parameters for now
}

export interface ArticleChromeMessage extends Omit<ChromeObj, 'type'> {
	type: ArticleMessageType;
}
