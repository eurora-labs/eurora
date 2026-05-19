import { YouTubeTranscriptApi, type FetchedTranscript } from './transcript/index.js';
import { createArticleAsset } from '../../../shared/content/extensions/article/util';
import {
	Watcher,
	type BrowserObj,
	type WatcherResponse,
} from '../../../shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { YoutubeBrowserMessage, WatcherParams } from './types.js';
import type { NativeYoutubeAsset, NativeYoutubeSnapshot } from '../../../shared/content/bindings';

interface EurImage {
	dataBase64: string;
	width: number;
	height: number;
}

/// Mirrors `eurora_tools_youtube::types::CurrentTimestamp`. The bridge
/// dispatcher decodes this shape verbatim — keep the field names and
/// types in sync with the Rust struct.
interface CurrentTimestampPayload {
	video_id: string;
	timestamp_seconds: number;
	duration_seconds: number;
	playing: boolean;
}

/// Mirrors `eurora_tools_youtube::types::TranscriptEntry`.
interface TranscriptEntryPayload {
	start_seconds: number;
	duration_seconds: number;
	text: string;
}

/// Mirrors `eurora_tools_youtube::types::Transcript`.
interface TranscriptPayload {
	video_id: string;
	language: string;
	entries: TranscriptEntryPayload[];
}

/// Mirrors `eurora_tools_youtube::types::CapturedFrame`.
interface CapturedFramePayload {
	video_id: string;
	timestamp_seconds: number;
	width: number;
	height: number;
	image_base64: string;
}

export class YoutubeWatcher extends Watcher<WatcherParams> {
	private youtubeTranscriptApi: YouTubeTranscriptApi;
	constructor(params: WatcherParams) {
		super(params);
		this.youtubeTranscriptApi = new YouTubeTranscriptApi();
	}

	private async fetchTranscript(): Promise<FetchedTranscript> {
		const videoId = getCurrentVideoId();
		if (videoId === undefined) {
			throw new Error('YouTube watch page has no video id');
		}
		return await this.youtubeTranscriptApi.fetch(videoId);
	}

	public listen(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
	): Promise<unknown> | false {
		const msg = obj as YoutubeBrowserMessage;
		if (msg.type === 'PLAY') {
			return this.handlePlay(msg, sender).catch(() => undefined);
		}
		switch (msg.type) {
			case 'GET_CURRENT_TIMESTAMP':
				return this.guard(this.handleGetCurrentTimestamp());
			case 'GET_TRANSCRIPT':
				return this.guard(this.handleGetTranscript());
			case 'GET_CURRENT_FRAME':
				return this.guard(this.handleGetCurrentFrame());
		}
		return super.listen(obj, sender);
	}

	public async handlePlay(
		obj: YoutubeBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<any> {
		const { value } = obj;
		if (this.params.youtubePlayer) {
			this.params.youtubePlayer.currentTime = value as number;
		}
	}

	public async handleNew(
		_obj: YoutubeBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return { kind: 'Ok', data: null };
	}

	private async generateVideoAsset(): Promise<any> {
		try {
			const currentTime = this.getCurrentVideoTime();
			const transcript = await this.fetchTranscript();

			const reportData: NativeYoutubeAsset = {
				url: window.location.href,
				title: document.title,
				transcript: JSON.stringify(transcript),
				current_time: Math.round(currentTime),
			};

			return { kind: 'NativeYoutubeAsset', data: reportData };
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error);
			console.error('Error generating YouTube report:', {
				url: window.location.href,
				videoId: getCurrentVideoId(),
				error: errorMessage,
				stack: error instanceof Error ? error.stack : undefined,
			});

			return {
				kind: 'Error',
				data: `Failed to generate YouTube assets for ${window.location.href}: ${errorMessage}`,
			};
		}
	}

	public async handleGenerateAssets(
		_obj: YoutubeBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		if (window.location.href.includes('/watch?v=')) {
			return await this.generateVideoAsset();
		} else {
			const articleAsset = createArticleAsset(document);
			return articleAsset;
		}
	}

	public async handleGenerateSnapshot(
		_obj: YoutubeBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		const currentTime = this.getCurrentVideoTime();
		const videoFrame = this.getCurrentVideoFrame();

		const reportData: NativeYoutubeSnapshot = {
			current_time: Math.round(currentTime),
			video_frame_base64: videoFrame.dataBase64,
			video_frame_width: videoFrame.width,
			video_frame_height: videoFrame.height,
		};

		return { kind: 'NativeYoutubeSnapshot', data: reportData };
	}

	/// Return the current playback state. Mirrors
	/// `eurora_tools_youtube::YoutubeAdapter::get_current_timestamp`.
	/// Throws if the page has no `<video>` element ready yet — the bridge
	/// catches that and returns it as a content-script error (HTTP 500
	/// in the bridge contract), not a tab-gone signal.
	public async handleGetCurrentTimestamp(): Promise<CurrentTimestampPayload> {
		const videoId = requireCurrentVideoId();
		const player = this.requirePlayer();
		return {
			video_id: videoId,
			timestamp_seconds: player.currentTime,
			duration_seconds: player.duration,
			playing: !player.paused,
		};
	}

	/// Return the active video's transcript. Mirrors
	/// `eurora_tools_youtube::YoutubeAdapter::get_transcript`. The
	/// language tag comes from YouTube's caption metadata — for
	/// auto-generated tracks this is the ASR language, for manual tracks
	/// the author-specified locale.
	public async handleGetTranscript(): Promise<TranscriptPayload> {
		const fetched = await this.fetchTranscript();
		return {
			video_id: fetched.videoId,
			language: fetched.languageCode,
			entries: fetched.snippets.map((s) => ({
				start_seconds: s.start,
				duration_seconds: s.duration,
				text: s.text,
			})),
		};
	}

	/// Capture the visible video frame as PNG. Mirrors
	/// `eurora_tools_youtube::YoutubeAdapter::get_current_frame`.
	public async handleGetCurrentFrame(): Promise<CapturedFramePayload> {
		const videoId = requireCurrentVideoId();
		const player = this.requirePlayer();
		const frame = this.getCurrentVideoFrame();
		return {
			video_id: videoId,
			timestamp_seconds: player.currentTime,
			width: frame.width,
			height: frame.height,
			image_base64: frame.dataBase64,
		};
	}

	private requirePlayer(): HTMLVideoElement {
		const player = this.getYouTubePlayer();
		if (!player) throw new Error('no YouTube player element on the page');
		if (player.readyState === 0) throw new Error('YouTube player not ready');
		return player;
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
		};
	}

	getCurrentVideoTime(): number {
		const player = this.getYouTubePlayer();
		if (!player) return -1.0;

		if (player.readyState === 0 || player.duration === 0) return -1.0;

		return player.currentTime;
	}

	getYouTubePlayer(): HTMLVideoElement | null {
		const { youtubePlayer } = this.params;
		if (!youtubePlayer) {
			this.params.youtubePlayer = document.querySelector(
				'video.html5-main-video',
			) as HTMLVideoElement;
		}
		return this.params.youtubePlayer;
	}
}

function getCurrentVideoId(): string | undefined {
	if (window.location.search?.includes('v=')) {
		return window.location.search.split('v=')[1].split('&')[0];
	}
	return undefined;
}

function requireCurrentVideoId(): string {
	const videoId = getCurrentVideoId();
	if (videoId === undefined) {
		throw new Error('YouTube watch page has no video id');
	}
	return videoId;
}

let initialized = false;

export function main() {
	if (initialized) return;
	initialized = true;

	const watcher = new YoutubeWatcher({
		canvas: document.createElement('canvas'),
		context: document.createElement('canvas').getContext('2d'),
		youtubePlayer: null,
	});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
