import { MessageType, BrowserObj } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';

type CustomMessageType = 'PLAY';
export type YoutubeMessageType = MessageType | CustomMessageType;

export interface WatcherParams {
	videoId?: string;
	videoTranscript?: any;
	canvas: HTMLCanvasElement;
	context: CanvasRenderingContext2D;
	youtubePlayer: HTMLVideoElement | null;
}

export interface YoutubeChromeMessage extends Omit<BrowserObj, 'type'> {
	type: YoutubeMessageType;
}
