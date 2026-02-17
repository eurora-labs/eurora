import type { BrowserObj } from '../../../shared/content/extensions/watchers/watcher';

export type CommonMessageType = 'GET_METADATA';

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface WatcherParams {
	// Common watcher doesn't need specific parameters for now
}

export interface CommonBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: CommonMessageType;
}
