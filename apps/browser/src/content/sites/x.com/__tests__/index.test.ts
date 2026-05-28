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

describe('x.com site handler', () => {
	beforeEach(() => {
		vi.clearAllMocks();
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
});
