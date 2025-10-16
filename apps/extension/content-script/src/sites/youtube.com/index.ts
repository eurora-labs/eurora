import {
	Watcher,
	type WatcherResponse,
} from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import { YoutubeChromeMessage, type WatcherParams } from './types.js';
import { YouTubeTranscriptApi } from './transcript/index.js';
import { ProtoImage, ProtoImageFormat } from '@eurora/shared/proto/shared_pb.js';
import { createArticleAsset } from '@eurora/chrome-ext-shared/extensions/article/util';
import type { NativeYoutubeAsset, NativeYoutubeSnapshot } from '@eurora/chrome-ext-shared/bindings';

interface EurImage extends Partial<ProtoImage> {
	dataBase64: string;
}

export class YoutubeWatcher extends Watcher<WatcherParams> {
	private youtubeTranscriptApi: YouTubeTranscriptApi;
	constructor(params: WatcherParams) {
		super(params);
		this.youtubeTranscriptApi = new YouTubeTranscriptApi();
	}

	private async ensureTranscript(videoId?: string): Promise<any> {
		if (!videoId) {
			videoId = this.params.videoId;
		}

		this.params.videoTranscript = (
			await this.youtubeTranscriptApi.fetch(videoId, ['en'])
		).snippets;
		return this.params.videoTranscript;
	}

	public listen(
		obj: YoutubeChromeMessage,
		sender: browser.Runtime.MessageSender,
		response: (response?: WatcherResponse) => void,
	) {
		const { type } = obj;
		let promise: Promise<WatcherResponse>;

		switch (type) {
			case 'NEW':
				promise = this.handleNew(obj, sender);
				break;
			case 'PLAY':
				promise = this.handlePlay(obj, sender);
				break;
			case 'GENERATE_ASSETS':
				promise = this.handleGenerateAssets(obj, sender);
				break;
			case 'GENERATE_SNAPSHOT':
				promise = this.handleGenerateSnapshot(obj, sender);
				break;
			default:
				response({ kind: 'Error', data: 'Invalid message type' });
				return false;
		}

		promise.then((result) => {
			response(result);
		});

		return true;
	}

	public async handlePlay(
		obj: YoutubeChromeMessage,
		sender: browser.Runtime.MessageSender,
	): Promise<any> {
		const { value } = obj;
		if (this.params.youtubePlayer) {
			this.params.youtubePlayer.currentTime = value as number;
		}
	}

	public async handleNew(
		obj: YoutubeChromeMessage,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		const currentVideoId = getCurrentVideoId();
		if (!currentVideoId) {
			this.params.videoId = undefined;
			this.params.videoTranscript = undefined;
			return;
		}
		this.params.videoId = currentVideoId;

		try {
			const transcript = await this.ensureTranscript(currentVideoId);
			this.params.videoTranscript = transcript;
		} catch (error) {
			console.error('Failed to get transcript:', error);
			browser.runtime.sendMessage({
				type: 'SEND_TO_NATIVE',
				payload: {
					videoId: this.params.videoId,
					error: error.message || 'Unknown error',
					transcript: null,
				},
			});
		}
	}

	private async generateVideoAsset(): Promise<any> {
		try {
			// Get current timestamp
			const currentTime = this.getCurrentVideoTime();
			const reportData: NativeYoutubeAsset = {
				url: window.location.href,
				title: document.title,
				transcript: this.params.videoTranscript
					? JSON.stringify(this.params.videoTranscript)
					: '',
				current_time: Math.round(currentTime),
			};

			if (reportData.transcript === '') {
				try {
					const transcript = await this.ensureTranscript();
					reportData.transcript = JSON.stringify(transcript);
					return { kind: 'NativeYoutubeAsset', data: reportData };
				} catch (error) {
					return {
						kind: 'Error',
						data: `Failed to get transcript: ${error.message}`,
					};
				}
			} else {
				return { kind: 'NativeYoutubeAsset', data: reportData };
			}
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error);
			const contextualError = `Failed to generate YouTube assets for ${window.location.href}: ${errorMessage}`;
			console.error('Error generating YouTube report:', {
				url: window.location.href,
				videoId: this.params.videoId,
				error: errorMessage,
				stack: error instanceof Error ? error.stack : undefined,
			});

			return {
				kind: 'Error',
				data: `Failed to generate YouTube assets: ${contextualError}`,
			};
		}
	}

	public async handleGenerateAssets(
		obj: YoutubeChromeMessage,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		if (window.location.href.includes('/watch?v=')) {
			return await this.generateVideoAsset();
		} else {
			const articleAsset = await createArticleAsset(document);
			return articleAsset;
		}
	}

	public async handleGenerateSnapshot(
		obj: YoutubeChromeMessage,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		const currentTime = this.getCurrentVideoTime();
		const videoFrame = this.getCurrentVideoFrame();

		const reportData: NativeYoutubeSnapshot = {
			current_time: Math.round(currentTime),
			video_frame_base64: videoFrame.dataBase64,
			video_frame_width: videoFrame.width,
			video_frame_height: videoFrame.height,
			// video_frame_format: videoFrame.format,
		};

		return { kind: 'NativeYoutubeSnapshot', data: reportData };
	}

	getCurrentVideoFrame(): EurImage {
		const { youtubePlayer, canvas } = this.params;
		if (!youtubePlayer) return null;

		canvas.width = youtubePlayer.videoWidth;
		canvas.height = youtubePlayer.videoHeight;

		canvas.getContext('2d')?.drawImage(youtubePlayer, 0, 0, canvas.width, canvas.height);

		return {
			dataBase64: canvas.toDataURL('image/png').split(',')[1],
			width: canvas.width,
			height: canvas.height,
			format: ProtoImageFormat.PNG,
		};
	}

	getCurrentVideoTime(): number {
		const player = this.getYouTubePlayer();
		if (!player) return -1.0;

		// Check if the video is actually loaded and playable
		if (player.readyState === 0 || player.duration === 0) return -1.0;

		return player.currentTime;
	}

	getYouTubePlayer(): HTMLVideoElement | null {
		const { youtubePlayer } = this.params;
		// Try to find the video element if we don't have it yet
		if (!youtubePlayer) {
			this.params.youtubePlayer = document.querySelector(
				'video.html5-main-video',
			) as HTMLVideoElement;
		}
		return this.params.youtubePlayer;
	}
}

function getCurrentVideoId() {
	if (window.location.search?.includes('v=')) {
		return window.location.search.split('v=')[1].split('&')[0];
	}
	return undefined;
}

export function main() {
	const watcher = new YoutubeWatcher({
		videoId: getCurrentVideoId(),
		videoTranscript: null,
		canvas: document.createElement('canvas'),
		context: document.createElement('canvas').getContext('2d'),
		youtubePlayer: null,
	});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
