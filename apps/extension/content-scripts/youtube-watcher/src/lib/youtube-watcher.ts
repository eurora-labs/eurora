import { Watcher } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { YoutubeChromeMessage, type WatcherParams } from './types.js';
import { YouTubeTranscriptApi } from './transcript/index.js';
import { ProtoImage, ProtoImageFormat } from '@eurora/shared/proto/shared_pb.js';
import { NativeYoutubeAsset, NativeYoutubeSnapshot } from '@eurora/chrome-ext-shared/bindings';

interface EurImage extends Partial<ProtoImage> {
	dataBase64: string;
}

class YoutubeWatcher extends Watcher<WatcherParams> {
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
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const { type } = obj;

		switch (type) {
			case 'NEW':
				this.handleNew(obj, sender, response);
				break;
			case 'PLAY':
				this.handlePlay(obj, sender, response);
				break;
			case 'GENERATE_ASSETS':
				this.handleGenerateAssets(obj, sender, response);
				break;
			case 'GENERATE_SNAPSHOT':
				this.handleGenerateSnapshot(obj, sender, response);
				break;
		}
	}

	public handlePlay(
		obj: YoutubeChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const { value } = obj;
		if (this.params.youtubePlayer) {
			this.params.youtubePlayer.currentTime = value as number;
		}
	}

	public handleNew(
		obj: YoutubeChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const currentVideoId = getCurrentVideoId();
		if (!currentVideoId) {
			this.params.videoId = undefined;
			this.params.videoTranscript = undefined;
			return;
		}
		this.params.videoId = currentVideoId;

		this.ensureTranscript(currentVideoId)
			.then((transcript) => {
				this.params.videoTranscript = transcript;
			})
			.catch((error) => {
				console.error('Failed to get transcript:', error);
				chrome.runtime.sendMessage({
					type: 'SEND_TO_NATIVE',
					payload: {
						videoId: this.params.videoId,
						error: error.message || 'Unknown error',
						transcript: null,
					},
				});
			});
	}

	public handleGenerateAssets(
		obj: YoutubeChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
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
			// const reportData = create(ProtoNativeYoutubeStateSchema, {
			// 	type: 'YOUTUBE_STATE',
			// 	url: window.location.href,
			// 	title: document.title,
			// 	transcript: this.params.videoTranscript
			// 		? JSON.stringify(this.params.videoTranscript)
			// 		: '',
			// 	currentTime: Math.round(currentTime),
			// });
			if (reportData.transcript === '') {
				this.ensureTranscript()
					.then((transcript) => {
						reportData.transcript = JSON.stringify(transcript);
						response({ kind: 'NativeYoutubeAsset', data: reportData });
					})
					.catch((error) => {
						response({
							success: false,
							error: `Failed to get transcript: ${error.message}`,
						});
					});
				return true;
			} else {
				response({ kind: 'NativeYoutubeAsset', data: reportData });
				return true;
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
			response({
				success: false,
				error: contextualError,
				context: {
					url: window.location.href,
					videoId: this.params.videoId,
					timestamp: new Date().toISOString(),
				},
			});
		}

		return true; // Important: indicates we'll send response asynchronously
	}

	public handleGenerateSnapshot(
		obj: YoutubeChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		console.log('Generating snapshots for YouTube video');
		const currentTime = this.getCurrentVideoTime();
		const videoFrame = this.getCurrentVideoFrame();

		const reportData: NativeYoutubeSnapshot = {
			current_time: Math.round(currentTime),
			video_frame_base64: videoFrame.dataBase64,
			video_frame_width: videoFrame.width,
			video_frame_height: videoFrame.height,
			// video_frame_format: videoFrame.format,
		};

		response({ kind: 'NativeYoutubeSnapshot', data: reportData });
		return true;
	}

	getCurrentVideoFrame(): EurImage {
		const { youtubePlayer, canvas } = this.params;
		if (!youtubePlayer) return null;

		canvas.width = youtubePlayer.videoWidth;
		canvas.height = youtubePlayer.videoHeight;

		canvas.getContext('2d')?.drawImage(youtubePlayer, 0, 0, canvas.width, canvas.height);

		// const link = document.createElement('a');
		// link.href = canvas.toDataURL('image/png');
		// link.download = 'youtube-snapshot.png';
		// link.click();

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

(() => {
	const watcher = new YoutubeWatcher({
		videoId: getCurrentVideoId(),
		videoTranscript: null,
		canvas: document.createElement('canvas'),
		context: document.createElement('canvas').getContext('2d'),
		youtubePlayer: null,
	});

	chrome.runtime.onMessage.addListener(watcher.listen.bind(watcher));
})();

function getCurrentVideoId() {
	if (window.location.search?.includes('v=')) {
		return window.location.search.split('v=')[1].split('&')[0];
	}
	return null;
}
