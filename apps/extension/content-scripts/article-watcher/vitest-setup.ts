import { vi } from 'vitest';
import '@testing-library/jest-dom';

// Mock Chrome API
const mockChrome = {
	runtime: {
		onMessage: {
			addListener: vi.fn(),
			removeListener: vi.fn(),
			hasListener: vi.fn(),
		},
		sendMessage: vi.fn(),
		connect: vi.fn(),
		getURL: vi.fn((path: string) => `chrome-extension://mock-id/${path}`),
		id: 'mock-extension-id',
	},
	storage: {
		local: {
			get: vi.fn(),
			set: vi.fn(),
			remove: vi.fn(),
			clear: vi.fn(),
		},
		sync: {
			get: vi.fn(),
			set: vi.fn(),
			remove: vi.fn(),
			clear: vi.fn(),
		},
	},
	tabs: {
		query: vi.fn(),
		sendMessage: vi.fn(),
		create: vi.fn(),
		update: vi.fn(),
	},
};

// Set up global chrome object
global.chrome = mockChrome as any;

// Mock console methods to reduce noise in tests
global.console = {
	...console,
	log: vi.fn(),
	debug: vi.fn(),
	info: vi.fn(),
	warn: vi.fn(),
	error: vi.fn(),
};
