import type {
	MessageType,
	BrowserObj,
} from '@eurora/browser-shared/content/extensions/watchers/watcher';

type CustomMessageType = 'PLAY';
export type YoutubeMessageType = MessageType | CustomMessageType;

export interface WatcherParams {
	videoId?: string;
	videoTranscript?: any;
	canvas: HTMLCanvasElement;
	context: CanvasRenderingContext2D;
	youtubePlayer: HTMLVideoElement | null;
}

export interface YoutubeBrowserMessage extends Omit<BrowserObj, 'type'> {
	type: YoutubeMessageType;
}
