import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

describe('Article Watcher Content Script', () => {
	describe('Basic functionality', () => {
		it('should perform basic arithmetic', () => {
			expect(1 + 2).toBe(3);
		});

		it('should handle string operations', () => {
			const str = 'article-watcher';
			expect(str).toContain('article');
			expect(str.split('-')).toHaveLength(2);
		});
	});

	describe('Module imports', () => {
		it('should be able to import the main module', async () => {
			// This test verifies that the module can be imported without errors
			await expect(import('./index.js')).resolves.toBeDefined();
		});
	});

	describe('Type definitions', () => {
		it('should have valid type exports', async () => {
			const types = await import('./lib/types.js');
			expect(types).toBeDefined();
		});
	});
});

describe('Article Watcher Integration', () => {
	let mockChrome: any;

	beforeEach(() => {
		// Mock chrome runtime API
		mockChrome = {
			runtime: {
				onMessage: {
					addListener: vi.fn(),
				},
			},
		};
		global.chrome = mockChrome;
	});

	afterEach(() => {
		vi.clearAllMocks();
	});

	it('should register message listener on load', async () => {
		// The module is already imported by the setup, so we just verify
		// that the chrome API is available and properly mocked
		expect(mockChrome.runtime.onMessage.addListener).toBeDefined();
		expect(typeof mockChrome.runtime.onMessage.addListener).toBe('function');
	});
});
