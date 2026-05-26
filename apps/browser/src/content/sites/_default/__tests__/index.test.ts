import { formatDefaultContext, main } from '../index.js';
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

describe('_default site handler', () => {
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

describe('formatDefaultContext', () => {
	const URL = 'https://example.com/article';

	it('reports title and url when nothing is selected', () => {
		expect(formatDefaultContext({ title: 'Example Article', url: URL, selection: '' })).toBe(
			`The user is on the web page "Example Article" at ${URL}.`,
		);
	});

	it('falls back to a url-only sentence when the title is empty', () => {
		expect(formatDefaultContext({ title: '', url: URL, selection: '' })).toBe(
			`The user is on the web page at ${URL}.`,
		);
	});

	it('appends a highlight clause when there is a selection', () => {
		expect(
			formatDefaultContext({
				title: 'Example Article',
				url: URL,
				selection: 'a notable sentence',
			}),
		).toBe(
			`The user is on the web page "Example Article" at ${URL}. They have the following text highlighted: "a notable sentence".`,
		);
	});

	it('appends a highlight clause to the url-only intro when the title is empty', () => {
		expect(
			formatDefaultContext({
				title: '',
				url: URL,
				selection: 'a notable sentence',
			}),
		).toBe(
			`The user is on the web page at ${URL}. They have the following text highlighted: "a notable sentence".`,
		);
	});

	it('omits the highlight clause entirely when the selection is empty', () => {
		const out = formatDefaultContext({ title: 'T', url: URL, selection: '' });
		expect(out).not.toContain('highlighted');
	});
});
