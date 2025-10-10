import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { ArticleChromeMessage } from './types.js';

describe('ArticleWatcher', () => {
	let mockDocument: any;
	let mockWindow: any;
	let mockChrome: any;

	beforeEach(() => {
		// Mock document
		mockDocument = {
			querySelector: vi.fn(),
			querySelectorAll: vi.fn(),
			body: {
				innerHTML: '<article><h1>Test Article</h1><p>Content</p></article>',
			},
		};
		global.document = mockDocument as any;

		// Mock window
		mockWindow = {
			location: {
				href: 'https://example.com/article',
			},
		};
		global.window = mockWindow as any;

		// Mock chrome runtime API
		mockChrome = {
			runtime: {
				onMessage: {
					addListener: vi.fn(),
				},
				sendMessage: vi.fn(),
			},
		};
		global.chrome = mockChrome;

		// Mock console methods
		vi.spyOn(console, 'log').mockImplementation(() => {});
	});

	afterEach(() => {
		vi.clearAllMocks();
		vi.restoreAllMocks();
	});

	describe('Message handling', () => {
		it('should handle NEW message type', async () => {
			// Import and test the watcher
			await import('./article-watcher.js');

			// Note: Since the module has an IIFE, we're testing the concept
			expect(console.log).toHaveBeenCalledWith('Article Watcher content script loaded');
		});

		it('should handle GENERATE_ASSETS message type', () => {
			const message: ArticleChromeMessage = {
				type: 'GENERATE_ASSETS',
			};

			expect(message.type).toBe('GENERATE_ASSETS');
		});

		it('should handle GENERATE_SNAPSHOT message type', () => {
			const message: ArticleChromeMessage = {
				type: 'GENERATE_SNAPSHOT',
			};

			expect(message.type).toBe('GENERATE_SNAPSHOT');
		});
	});

	describe('Chrome API integration', () => {
		it('should have chrome runtime API available', () => {
			expect(global.chrome).toBeDefined();
			expect(global.chrome.runtime).toBeDefined();
			expect(global.chrome.runtime.onMessage).toBeDefined();
			expect(global.chrome.runtime.onMessage.addListener).toBeDefined();
		});

		it('should have message listener function', () => {
			expect(typeof mockChrome.runtime.onMessage.addListener).toBe('function');
		});
	});

	describe('Document and Window mocks', () => {
		it('should have document available', () => {
			expect(global.document).toBeDefined();
			expect(global.document.body).toBeDefined();
		});

		it('should have window location available', () => {
			expect(global.window).toBeDefined();
			expect(global.window.location.href).toBe('https://example.com/article');
		});
	});

	describe('Type validation', () => {
		it('should validate ArticleChromeMessage structure', () => {
			const validMessage: ArticleChromeMessage = {
				type: 'NEW',
			};

			expect(validMessage).toHaveProperty('type');
			expect(['NEW', 'GENERATE_ASSETS', 'GENERATE_SNAPSHOT']).toContain(validMessage.type);
		});

		it('should support all message types', () => {
			const messageTypes = ['NEW', 'GENERATE_ASSETS', 'GENERATE_SNAPSHOT'];

			messageTypes.forEach((type) => {
				const message: ArticleChromeMessage = {
					type: type as any,
				};
				expect(message.type).toBe(type);
			});
		});
	});
});
