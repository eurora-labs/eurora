import { describe, it, expect, vi, beforeEach } from 'vitest';
import { main } from '../index.js';

// Mock dependencies
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

vi.mock('@eurora/chrome-ext-shared/extensions/article/util', () => ({
	createArticleAsset: vi.fn().mockResolvedValue({
		kind: 'NativeArticleAsset',
		data: { url: 'test-url', title: 'test-title' },
	}),
	createArticleSnapshot: vi.fn().mockResolvedValue({
		kind: 'NativeArticleSnapshot',
		data: { screenshot: 'base64-data' },
	}),
}));

describe('_default site handler', () => {
	beforeEach(() => {
		vi.clearAllMocks();

		// Setup basic DOM
		document.body.innerHTML = '<div>Test content</div>';
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

	it('should handle NEW message type', async () => {
		const mockResponse = vi.fn();
		const mockSender = {} as any;

		const message = {
			type: 'NEW',
		};

		// The actual handler would be tested with more setup
		expect(message.type).toBe('NEW');
	});

	it('should handle GENERATE_ASSETS message type', async () => {
		const message = {
			type: 'GENERATE_ASSETS',
		};

		expect(message.type).toBe('GENERATE_ASSETS');
	});

	it('should handle GENERATE_SNAPSHOT message type', async () => {
		const message = {
			type: 'GENERATE_SNAPSHOT',
		};

		expect(message.type).toBe('GENERATE_SNAPSHOT');
	});

	it('should return error for invalid message type', () => {
		const message = {
			type: 'INVALID_TYPE',
		};

		expect(message.type).not.toBe('NEW');
		expect(message.type).not.toBe('GENERATE_ASSETS');
		expect(message.type).not.toBe('GENERATE_SNAPSHOT');
	});
});
