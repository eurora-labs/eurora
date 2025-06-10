import { MessageType, ChromeMessage } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';

type CustomMessageType = 'PLAY';
export type YoutubeMessageType = MessageType | CustomMessageType;
export class YoutubeChromeMessage implements ChromeMessage {
	message: any & { type: YoutubeMessageType };
	sender: chrome.runtime.MessageSender;
	response: (response?: any) => void;
}

export interface WatcherParams {
	videoId?: string;
	videoTranscript?: any;
	canvas: HTMLCanvasElement;
	context: CanvasRenderingContext2D;
	youtubePlayer: HTMLVideoElement | null;
}
