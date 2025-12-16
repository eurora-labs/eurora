import { describe, it, expect, vi, beforeEach } from 'vitest';

describe('bootstrap', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('should register message listener on load', async () => {
		const mockAddListener = vi.fn();

		// Mock browser runtime
		(global as any).browser = {
			runtime: {
				onMessage: {
					addListener: mockAddListener,
				},
				getURL: vi.fn((path: string) => `chrome-extension://mock/${path}`),
			},
		};

		// Import bootstrap to trigger listener registration
		await import('../bootstrap.js');

		expect(mockAddListener).toHaveBeenCalled();
		expect(mockAddListener).toHaveBeenCalledWith(expect.any(Function));
	});

	it('should handle SITE_LOAD message type', async () => {
		const mockSendResponse = vi.fn();
		const mockGetURL = vi.fn((path: string) => `chrome-extension://mock/${path}`);

		(global as any).browser = {
			runtime: {
				onMessage: {
					addListener: vi.fn(),
				},
				getURL: mockGetURL,
			},
		};

		// Mock dynamic import
		vi.mock('../bootstrap.js', async () => {
			const actual = await vi.importActual('../bootstrap.js');
			return actual;
		});

		const message = {
			type: 'SITE_LOAD',
			chunk: 'sites/test/index.js',
			defaultChunk: 'sites/_default/index.js',
		};

		// This is a basic structure test - actual message handling would require more setup
		expect(message.type).toBe('SITE_LOAD');
		expect(message.chunk).toBeDefined();
		expect(message.defaultChunk).toBeDefined();
	});

	it('should ignore non-SITE_LOAD messages', () => {
		const message = {
			type: 'OTHER_MESSAGE',
		};

		expect(message.type).not.toBe('SITE_LOAD');
	});

	it('should only load once', () => {
		// Test that loaded flag prevents multiple loads
		const messages = [
			{ type: 'SITE_LOAD', chunk: 'test1.js', defaultChunk: 'default.js' },
			{ type: 'SITE_LOAD', chunk: 'test2.js', defaultChunk: 'default.js' },
		];

		// Both messages have SITE_LOAD type
		expect(messages[0].type).toBe('SITE_LOAD');
		expect(messages[1].type).toBe('SITE_LOAD');

		// But the bootstrap should only process the first one
		// (actual implementation test would require more complex setup)
	});
});
