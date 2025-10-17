import { vi } from 'vitest';
import '@testing-library/jest-dom';

// Mock webextension-polyfill
const mockBrowser = {
	runtime: {
		onMessage: {
			addListener: vi.fn(),
			removeListener: vi.fn(),
		},
		sendMessage: vi.fn(),
		getURL: vi.fn((path: string) => `chrome-extension://mock-id/${path}`),
	},
};

vi.mock('webextension-polyfill', () => ({
	default: mockBrowser,
}));

// Mock chrome API
global.chrome = {
	runtime: {
		onMessage: {
			addListener: vi.fn(),
			removeListener: vi.fn(),
		},
		sendMessage: vi.fn(),
		getURL: vi.fn((path: string) => `chrome-extension://mock-id/${path}`),
	},
} as any;

// Mock browser API
(global as any).browser = mockBrowser;

// Setup DOM environment
global.document = window.document;
global.navigator = window.navigator;

// Mock canvas context for jsdom
HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
	drawImage: vi.fn(),
	fillRect: vi.fn(),
	clearRect: vi.fn(),
	getImageData: vi.fn(),
	putImageData: vi.fn(),
	createImageData: vi.fn(),
	setTransform: vi.fn(),
	resetTransform: vi.fn(),
	canvas: document.createElement('canvas'),
})) as any;

HTMLCanvasElement.prototype.toDataURL = vi.fn(() => 'data:image/png;base64,mockdata');
