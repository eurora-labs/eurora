import { Watcher } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { YoutubeChromeMessage, type WatcherParams } from './types.js';
import { YouTubeTranscriptApi } from '@eurora/youtube-transcripts';
import { ProtoImage, ProtoImageFormat } from '@eurora/proto/shared';
import { create } from '@eurora/proto/util.js';
import {
	ProtoNativeYoutubeState,
	ProtoNativeYoutubeSnapshot,
	ProtoNativeYoutubeStateSchema,
	ProtoNativeYoutubeSnapshotSchema,
} from '@eurora/proto/native_messaging';

interface EurImage extends Partial<ProtoImage> {
	dataBase64: string;
}
class YoutubeWatcher extends Watcher<WatcherParams> {
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
			this.params.youtubePlayer.currentTime = value;
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

		getYouTubeTranscript(currentVideoId)
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

			const videoFrame = this.getCurrentVideoFrame();
			const reportData = create(ProtoNativeYoutubeStateSchema, {
				type: 'YOUTUBE_STATE',
				url: window.location.href,
				title: document.title,
				transcript: JSON.stringify(this.params.videoTranscript),
				currentTime: Math.round(currentTime),
				videoFrameBase64: videoFrame.dataBase64,
				videoFrameWidth: videoFrame.width,
				videoFrameHeight: videoFrame.height,
				videoFrameFormat: videoFrame.format,
			});

			if (!this.params.videoTranscript) {
				getYouTubeTranscript(this.params.videoId).then((transcript) => {
					this.params.videoTranscript = transcript;
					// Prepare report data
					reportData.transcript = JSON.stringify(this.params.videoTranscript);

					// Send response back to background script
					response(reportData);
				});
			} else {
				response(reportData);
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

		const reportData = create(ProtoNativeYoutubeSnapshotSchema, {
			type: 'YOUTUBE_SNAPSHOT',
			currentTime: Math.round(currentTime),
			videoFrameBase64: videoFrame.dataBase64,
			videoFrameWidth: videoFrame.width,
			videoFrameHeight: videoFrame.height,
			videoFrameFormat: videoFrame.format,
		});

		response(reportData);
		return true;
	}

	getCurrentVideoFrame(): EurImage {
		const { youtubePlayer, canvas, context } = this.params;
		if (!youtubePlayer) return null;

		context.drawImage(youtubePlayer, 0, 0, canvas.width, canvas.height);

		return {
			dataBase64: canvas.toDataURL('image/jpeg').split(',')[1],
			width: canvas.width,
			height: canvas.height,
			format: ProtoImageFormat.JPEG,
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
	// let videoId = getCurrentVideoId();
	// let videoTranscript = null;
	// let canvas = document.createElement('canvas');
	// let context = canvas.getContext('2d');

	// if (!videoId) return;

	// // Make sure we get the YouTube player element
	// let youtubePlayer: HTMLVideoElement | null = null;

	// Function to initialize/get the YouTube player

	// Function to get current timestamp (or -1 if no video playing)

	// async function sendTranscriptToBackground(transcript: any) {
	// 	chrome.runtime.sendMessage(
	// 		{
	// 			type: 'SEND_TO_NATIVE',
	// 			payload: {
	// 				videoId,
	// 				transcript,
	// 			},
	// 		},
	// 		(response) => {
	// 			if (chrome.runtime.lastError) {
	// 				console.error('Error sending transcript:', chrome.runtime.lastError);
	// 			} else if (response) {
	// 				console.log('Transcript sent successfully, response:', response);
	// 			}
	// 		},
	// 	);
	// }

	let youtubeLeftControls: HTMLElement;

	chrome.runtime.onMessage.addListener(watcher.listen.bind(watcher));

	// Listen for messages from the extension
	// chrome.runtime.onMessage.addListener((obj, sender, response) => {
	// 	const { type, value, videoId: msgVideoId } = obj;

	// 	if (type === 'NEW') {
	// 		videoId = getCurrentVideoId();
	// 		if (!videoId) return;
	// 		getYouTubeTranscript(videoId)
	// 			.then((transcript) => {
	// 				videoTranscript = transcript;
	// 				// sendTranscriptToBackground(transcript);
	// 			})
	// 			.catch((error) => {
	// 				console.error('Failed to get transcript:', error);
	// 				// Notify service worker of failure
	// 				chrome.runtime.sendMessage({
	// 					type: 'SEND_TO_NATIVE',
	// 					payload: {
	// 						videoId,
	// 						error: error.message || 'Unknown error',
	// 						transcript: null,
	// 					},
	// 				});
	// 			});
	// 	} else if (type === 'PLAY') {
	// 		const player = getYouTubePlayer();
	// 		if (player) {
	// 			player.currentTime = value;
	// 		}
	// 	} else if (type === 'GENERATE_ASSETS') {
	// 		console.log('Generating assets for YouTube video');
	// 		try {
	// 			// Get current timestamp
	// 			const currentTime = getCurrentVideoTime();

	// 			const videoFrame = getCurrentVideoFrame();
	// 			const reportData: ProtoNativeYoutubeState = {
	// 				type: 'YOUTUBE_STATE',
	// 				url: window.location.href,
	// 				title: document.title,
	// 				transcript: JSON.stringify(videoTranscript),
	// 				currentTime: Math.round(currentTime),
	// 				videoFrameBase64: videoFrame.dataBase64,
	// 				videoFrameWidth: videoFrame.width,
	// 				videoFrameHeight: videoFrame.height,
	// 				videoFrameFormat: videoFrame.format,
	// 			};

	// 			if (!videoTranscript) {
	// 				getYouTubeTranscript(videoId).then((transcript) => {
	// 					videoTranscript = transcript;
	// 					// Prepare report data
	// 					reportData.transcript = JSON.stringify(videoTranscript);

	// 					// Send response back to background script
	// 					response(reportData);
	// 				});
	// 			} else {
	// 				response(reportData);
	// 			}
	// 		} catch (error) {
	// 			const errorMessage = error instanceof Error ? error.message : String(error);
	// 			const contextualError = `Failed to generate YouTube assets for ${window.location.href}: ${errorMessage}`;
	// 			console.error('Error generating YouTube report:', {
	// 				url: window.location.href,
	// 				videoId: videoId,
	// 				error: errorMessage,
	// 				stack: error instanceof Error ? error.stack : undefined,
	// 			});
	// 			response({
	// 				success: false,
	// 				error: contextualError,
	// 				context: {
	// 					url: window.location.href,
	// 					videoId: videoId,
	// 					timestamp: new Date().toISOString(),
	// 				},
	// 			});
	// 		}

	// 		return true; // Important: indicates we'll send response asynchronously
	// 	} else if (type === 'GENERATE_SNAPSHOT') {
	// 		console.log('Generating snapshots for YouTube video');
	// 		const currentTime = getCurrentVideoTime();
	// 		const videoFrame = getCurrentVideoFrame();

	// 		const reportData: ProtoNativeYoutubeSnapshot = {
	// 			type: 'YOUTUBE_SNAPSHOT',
	// 			currentTime: Math.round(currentTime),
	// 			videoFrameBase64: videoFrame.dataBase64,
	// 			videoFrameWidth: videoFrame.width,
	// 			videoFrameHeight: videoFrame.height,
	// 			videoFrameFormat: videoFrame.format,
	// 		};

	// 		response(reportData);
	// 		return true;
	// 	}

	// 	// For non-async handlers
	// 	if (type !== 'GENERATE_ASSETS' && type !== 'GENERATE_SNAPSHOT') {
	// 		response();
	// 	}
	// });

	// Initialize player reference when script loads
	// getYouTubePlayer();
})();

function getCurrentVideoId() {
	if (window.location.search?.includes('v=')) {
		return window.location.search.split('v=')[1].split('&')[0];
	}
	return null;
}

async function getYouTubeTranscript(videoId: string) {
	return await YouTubeTranscriptApi.getTranscript(videoId);
}
