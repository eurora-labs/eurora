import type {
	MessageType,
	BrowserObj,
} from '@eurora/browser-shared/content/extensions/watchers/watcher';

export type ArticleMessageType = MessageType;

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface WatcherParams {
	// Article watcher doesn't need specific parameters for now
}

export interface ArticleBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: ArticleMessageType;
}
