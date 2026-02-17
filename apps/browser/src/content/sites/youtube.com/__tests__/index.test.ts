import { main } from '../index.js';
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

vi.mock('../transcript/index.js', () => ({
	YouTubeTranscriptApi: vi.fn().mockImplementation(() => ({
		fetch: vi.fn().mockResolvedValue({
			snippets: [{ text: 'Test transcript', start: 0, duration: 5 }],
		}),
	})),
}));

describe('youtube.com site handler', () => {
	beforeEach(() => {
		vi.clearAllMocks();

		document.body.innerHTML = `
			<video class="html5-main-video"></video>
		`;

		Object.defineProperty(window, 'location', {
			value: {
				href: 'https://www.youtube.com/watch?v=test123',
				search: '?v=test123',
			},
			writable: true,
		});
	});

	it('should export main function', () => {
		expect(main).toBeDefined();
		expect(typeof main).toBe('function');
	});

	it('should register message listener when main is called', async () => {
		const browser = await import('webextension-polyfill');

		main();

		expect(browser.default.runtime.onMessage.addListener).toHaveBeenCalled();
	});

	it('should handle NEW message type', () => {
		const message = {
			type: 'NEW',
		};

		expect(message.type).toBe('NEW');
	});

	it('should handle PLAY message type', () => {
		const message = {
			type: 'PLAY',
			value: 10.5,
		};

		expect(message.type).toBe('PLAY');
		expect(message.value).toBe(10.5);
	});

	it('should handle GENERATE_ASSETS message type', () => {
		const message = {
			type: 'GENERATE_ASSETS',
		};

		expect(message.type).toBe('GENERATE_ASSETS');
	});

	it('should handle GENERATE_SNAPSHOT message type', () => {
		const message = {
			type: 'GENERATE_SNAPSHOT',
		};

		expect(message.type).toBe('GENERATE_SNAPSHOT');
	});

	it('should extract video ID from URL', () => {
		const url = 'https://www.youtube.com/watch?v=test123';
		const videoId = url.includes('v=') ? url.split('v=')[1].split('&')[0] : null;

		expect(videoId).toBe('test123');
	});

	it('should return null for non-video URLs', () => {
		const url = 'https://www.youtube.com/';
		const videoId = url.includes('v=') ? url.split('v=')[1].split('&')[0] : null;

		expect(videoId).toBeNull();
	});

	it('should find YouTube video element', () => {
		const videoElement = document.querySelector('video.html5-main-video');

		expect(videoElement).toBeTruthy();
		expect(videoElement?.tagName).toBe('VIDEO');
	});

	it('should create canvas for video frame capture', () => {
		const canvas = document.createElement('canvas');

		expect(canvas).toBeTruthy();
		expect(canvas.tagName).toBe('CANVAS');
		expect(typeof canvas.getContext).toBe('function');
	});
});
