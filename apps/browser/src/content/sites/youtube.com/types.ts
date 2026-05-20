import type { MessageType, BrowserObj } from '../../../shared/content/extensions/watchers/watcher';

type CustomMessageType = 'GET_CURRENT_TIMESTAMP' | 'GET_TRANSCRIPT' | 'GET_CURRENT_FRAME';
export type YoutubeMessageType = MessageType | CustomMessageType;

export interface WatcherParams {
	canvas: HTMLCanvasElement;
	youtubePlayer: HTMLVideoElement | null;
}

export interface YoutubeBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: YoutubeMessageType;
}
