import { YoutubeWatcher, main } from '../index.js';
import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('webextension-polyfill', () => ({
	default: {
		runtime: {
			onMessage: {
				addListener: vi.fn(),
			},
			sendMessage: vi.fn(),
		},
	},
}));

vi.mock('../../../../shared/content/extensions/article/util', () => ({
	createArticleAsset: vi.fn().mockResolvedValue({
		kind: 'NativeArticleAsset',
		data: { url: 'test-url', title: 'test-title' },
	}),
}));

const transcriptFetchMock = vi.fn();
vi.mock('../transcript/index.js', () => ({
	YouTubeTranscriptApi: vi.fn().mockImplementation(() => ({
		fetch: transcriptFetchMock,
	})),
}));

function makeWatcher() {
	const canvas = document.createElement('canvas');
	const context = canvas.getContext('2d') as CanvasRenderingContext2D;
	const player = document.querySelector('video.html5-main-video') as HTMLVideoElement;
	// jsdom doesn't run the video pipeline; stub the bits the handlers
	// read directly so we can drive them without a real media element.
	Object.defineProperty(player, 'currentTime', { value: 12.5, configurable: true });
	Object.defineProperty(player, 'duration', { value: 240.0, configurable: true });
	Object.defineProperty(player, 'paused', { value: false, configurable: true });
	Object.defineProperty(player, 'readyState', { value: 4, configurable: true });
	Object.defineProperty(player, 'videoWidth', { value: 640, configurable: true });
	Object.defineProperty(player, 'videoHeight', { value: 360, configurable: true });
	return new YoutubeWatcher({ canvas, context, youtubePlayer: player });
}

function makeSender(): import('webextension-polyfill').Runtime.MessageSender {
	return {} as import('webextension-polyfill').Runtime.MessageSender;
}

describe('youtube.com site handler', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		transcriptFetchMock.mockReset();

		document.body.innerHTML = `
			<video class="html5-main-video"></video>
		`;

		Object.defineProperty(window, 'location', {
			value: {
				href: 'https://www.youtube.com/watch?v=abc123',
				search: '?v=abc123',
			},
			writable: true,
		});
	});

	it('registers a message listener when main is called', async () => {
		const browser = await import('webextension-polyfill');
		main();
		expect(browser.default.runtime.onMessage.addListener).toHaveBeenCalled();
	});

	describe('GET_CURRENT_TIMESTAMP', () => {
		it('returns the typed CurrentTimestamp payload', async () => {
			const watcher = makeWatcher();
			const result = await watcher.listen({ type: 'GET_CURRENT_TIMESTAMP' }, makeSender());

			expect(result).toEqual({
				video_id: 'abc123',
				timestamp_seconds: 12.5,
				duration_seconds: 240.0,
				playing: true,
			});
		});

		it('surfaces a missing player as a guarded {kind: Error} envelope', async () => {
			document.body.innerHTML = '';
			const watcher = new YoutubeWatcher({
				canvas: document.createElement('canvas'),
				context: document.createElement('canvas').getContext('2d'),
				youtubePlayer: null,
			});
			const result = await watcher.listen({ type: 'GET_CURRENT_TIMESTAMP' }, makeSender());
			expect(result).toMatchObject({ kind: 'Error' });
		});
	});

	describe('GET_TRANSCRIPT', () => {
		it('returns the typed Transcript payload with snippets mapped to entries', async () => {
			transcriptFetchMock.mockResolvedValueOnce({
				videoId: 'abc123',
				languageCode: 'en',
				snippets: [
					{ text: 'Hello', start: 0, duration: 1.5 },
					{ text: 'World', start: 1.5, duration: 1.0 },
				],
			});

			const watcher = makeWatcher();
			const result = await watcher.listen({ type: 'GET_TRANSCRIPT' }, makeSender());

			expect(result).toEqual({
				video_id: 'abc123',
				language: 'en',
				entries: [
					{ start_seconds: 0, duration_seconds: 1.5, text: 'Hello' },
					{ start_seconds: 1.5, duration_seconds: 1.0, text: 'World' },
				],
			});
		});

		it('surfaces transcript fetch failures as a guarded error envelope', async () => {
			transcriptFetchMock.mockRejectedValueOnce(new Error('no captions'));
			const watcher = makeWatcher();
			const result = await watcher.listen({ type: 'GET_TRANSCRIPT' }, makeSender());
			expect(result).toMatchObject({ kind: 'Error', data: 'no captions' });
		});
	});

	describe('GET_CURRENT_FRAME', () => {
		it('returns the typed CapturedFrame payload', async () => {
			const watcher = makeWatcher();
			const result = await watcher.listen({ type: 'GET_CURRENT_FRAME' }, makeSender());

			expect(result).toEqual({
				video_id: 'abc123',
				timestamp_seconds: 12.5,
				width: 640,
				height: 360,
				image_base64: 'mockdata',
			});
		});
	});

	describe('unrelated messages', () => {
		it('delegates GENERATE_ASSETS to the existing watcher path on watch URLs', async () => {
			transcriptFetchMock.mockResolvedValueOnce({
				videoId: 'abc123',
				languageCode: 'en',
				snippets: [],
			});
			const watcher = makeWatcher();
			const result = await watcher.listen({ type: 'GENERATE_ASSETS' }, makeSender());
			expect(result).toMatchObject({ kind: 'NativeYoutubeAsset' });
		});

		it('returns false for unknown message types', async () => {
			const watcher = makeWatcher();
			const result = watcher.listen({ type: 'UNKNOWN' as never }, makeSender());
			expect(result).toBe(false);
		});
	});
});
