import { MessageType, ChromeObj } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';

export type ArticleMessageType = MessageType;

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface WatcherParams {
	// Article watcher doesn't need specific parameters for now
}

export interface ArticleChromeMessage extends Omit<ChromeObj, 'type'> {
	type: ArticleMessageType;
}
