import type { MessageType, BrowserObj } from '../../../shared/content/extensions/watchers/watcher';

export type ArticleMessageType = MessageType;

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface WatcherParams {}

export interface ArticleBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: ArticleMessageType;
}
