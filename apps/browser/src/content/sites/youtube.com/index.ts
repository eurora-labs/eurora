import { YouTubeTranscriptApi, type FetchedTranscript } from './transcript/index.js';
import { createArticleAsset } from '../../../shared/content/extensions/article/util';
import {
	Watcher,
	type BrowserObj,
	type WatcherResponse,
} from '../../../shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { YoutubeBrowserMessage, WatcherParams } from './types.js';
import type {
	CapturedFrame,
	CurrentTimestamp,
	NativeYoutubeAsset,
	Transcript,
} from '../../../shared/content/bindings';

/// Raw frame capture without the per-call envelope. Used internally by
/// both `handleGetCurrentFrame` (tool path) and `handleGenerateSnapshot`
/// (legacy activity-capture path), so the canvas-draw logic exists in
/// exactly one place.
interface RawFrame {
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

	public listen(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
	): Promise<unknown> | false {
		const msg = obj as YoutubeBrowserMessage;
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

	public async handleNew(
		_obj: YoutubeBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return { kind: 'Ok', data: null };
	}

	public async handleGenerateAssets(
		_obj: YoutubeBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		if (!window.location.href.includes('/watch?v=')) {
			return createArticleAsset(document);
		}

		try {
			const [{ current_time }, transcript] = await Promise.all([
				this.handleGetCurrentTimestamp(),
				this.handleGetTranscript(),
			]);

			const reportData: NativeYoutubeAsset = {
				url: window.location.href,
				title: document.title,
				transcript: transcript.entries,
				current_time,
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

	public async handleGenerateSnapshot(
		_obj: YoutubeBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		// The activity-pipeline snapshot now ships the canonical
		// `CapturedFrame` shape — same as the
		// `browser::youtube::get_current_frame` tool — so there's a single
		// representation of "a YouTube frame at a moment in time" across
		// the bridge.
		const reportData: CapturedFrame = await this.handleGetCurrentFrame();
		return { kind: 'NativeYoutubeSnapshot', data: reportData };
	}

	/// Return the current playback state. Throws if the page has no
	/// `<video>` element ready yet — the bridge catches that and returns
	/// it as a content-script error (HTTP 500 in the bridge contract),
	/// not a tab-gone signal.
	public async handleGetCurrentTimestamp(): Promise<CurrentTimestamp> {
		const videoId = requireCurrentVideoId();
		const player = this.requirePlayer();
		return {
			video_id: videoId,
			current_time: player.currentTime,
			duration: player.duration,
			playing: !player.paused,
		};
	}

	/// Return the active video's transcript. The language tag comes from
	/// YouTube's caption metadata — for auto-generated tracks this is the
	/// ASR language, for manual tracks the author-specified locale.
	public async handleGetTranscript(): Promise<Transcript> {
		const fetched = await this.fetchTranscript();
		return {
			video_id: fetched.videoId,
			language: fetched.languageCode,
			entries: fetched.snippets.map((s) => ({
				start: s.start,
				duration: s.duration,
				text: s.text,
			})),
		};
	}

	/// Capture the visible video frame as PNG.
	public async handleGetCurrentFrame(): Promise<CapturedFrame> {
		const videoId = requireCurrentVideoId();
		const player = this.requirePlayer();
		const frame = this.captureFrame(player);
		return {
			video_id: videoId,
			current_time: player.currentTime,
			width: frame.width,
			height: frame.height,
			image_base64: frame.image_base64,
		};
	}

	private async fetchTranscript(): Promise<FetchedTranscript> {
		return await this.youtubeTranscriptApi.fetch(requireCurrentVideoId());
	}

	/// Resolve the `<video>` element. Throws when the page has no player
	/// or the player hasn't loaded enough data to read `currentTime` /
	/// `duration` (`readyState === 0`). Every primitive that reads from
	/// the player goes through here so the not-ready and not-on-page
	/// failures show up as the same structured content-script error.
	private requirePlayer(): HTMLVideoElement {
		const cached = this.params.youtubePlayer;
		const player = cached ?? document.querySelector<HTMLVideoElement>('video.html5-main-video');
		if (!player) throw new Error('no YouTube player element on the page');
		if (player.readyState === 0) throw new Error('YouTube player not ready');
		this.params.youtubePlayer = player;
		return player;
	}

	/// Encode the current video frame as base64 PNG. Returns the dimensions
	/// of the source video stream rather than the canvas (which can differ
	/// if the player has been letterboxed).
	private captureFrame(player: HTMLVideoElement): RawFrame {
		const { canvas } = this.params;
		canvas.width = player.videoWidth;
		canvas.height = player.videoHeight;
		const ctx = canvas.getContext('2d');
		if (!ctx) throw new Error('2D canvas context unavailable');
		ctx.drawImage(player, 0, 0, canvas.width, canvas.height);
		return {
			image_base64: canvas.toDataURL('image/png').split(',')[1],
			width: canvas.width,
			height: canvas.height,
		};
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
		youtubePlayer: null,
	});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
