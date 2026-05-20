import { YouTubeTranscriptApi, type FetchedTranscript } from './transcript/index.js';
import {
	Watcher,
	type BrowserObj,
	type WatcherResponse,
} from '../../../shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { YoutubeBrowserMessage, WatcherParams } from './types.js';
import type { CapturedFrame, CurrentTimestamp, Transcript } from '../../../shared/content/bindings';

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
		const { canvas } = this.params;
		canvas.width = player.videoWidth;
		canvas.height = player.videoHeight;
		const ctx = canvas.getContext('2d');
		if (!ctx) throw new Error('2D canvas context unavailable');
		ctx.drawImage(player, 0, 0, canvas.width, canvas.height);
		return {
			video_id: videoId,
			current_time: player.currentTime,
			width: canvas.width,
			height: canvas.height,
			image_base64: canvas.toDataURL('image/png').split(',')[1],
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
