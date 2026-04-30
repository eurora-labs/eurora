import {
	collectIconCandidatesFromLinks,
	fetchIconAsBase64,
	isIconRel,
	isSupportedScheme,
	originFallbackCandidate,
	parseSizes,
	rankCandidates,
	resolveBestCandidate,
	tabFaviconCandidate,
	type IconCandidate,
	type IconLinkRecord,
} from '../favicon-ranker';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

function candidate(partial: Partial<IconCandidate>): IconCandidate {
	return {
		href: 'https://example.com/icon.png',
		rel: 'icon',
		type: 'image/png',
		size: 0,
		source: 'dom',
		order: 0,
		...partial,
	};
}

describe('parseSizes', () => {
	it('returns Infinity for SVG type', () => {
		expect(parseSizes('', 'image/svg+xml')).toBe(Number.POSITIVE_INFINITY);
	});

	it('returns Infinity for sizes="any"', () => {
		expect(parseSizes('any', 'image/png')).toBe(Number.POSITIVE_INFINITY);
	});

	it('parses single size token', () => {
		expect(parseSizes('32x32', 'image/png')).toBe(32);
	});

	it('returns largest dimension across multiple tokens', () => {
		expect(parseSizes('16x16 32x32 192x192', 'image/png')).toBe(192);
	});

	it('uses smaller dimension when tokens are non-square', () => {
		expect(parseSizes('100x200', 'image/png')).toBe(100);
	});

	it('returns 0 when sizes attr is empty/missing', () => {
		expect(parseSizes('', 'image/png')).toBe(0);
		expect(parseSizes(null, 'image/png')).toBe(0);
		expect(parseSizes(undefined, 'image/png')).toBe(0);
	});

	it('ignores malformed tokens', () => {
		expect(parseSizes('not-a-size', 'image/png')).toBe(0);
		expect(parseSizes('32', 'image/png')).toBe(0);
	});
});

describe('isIconRel', () => {
	it('accepts standard icon rels', () => {
		expect(isIconRel('icon')).toBe(true);
		expect(isIconRel('shortcut icon')).toBe(true);
		expect(isIconRel('apple-touch-icon')).toBe(true);
		expect(isIconRel('apple-touch-icon-precomposed')).toBe(true);
		expect(isIconRel('mask-icon')).toBe(true);
		expect(isIconRel('fluid-icon')).toBe(true);
	});

	it('is case-insensitive', () => {
		expect(isIconRel('ICON')).toBe(true);
		expect(isIconRel('Apple-Touch-Icon')).toBe(true);
	});

	it('rejects non-icon rels', () => {
		expect(isIconRel('stylesheet')).toBe(false);
		expect(isIconRel('canonical')).toBe(false);
		expect(isIconRel('alternate')).toBe(false);
		expect(isIconRel('')).toBe(false);
	});
});

describe('isSupportedScheme', () => {
	it('accepts http/https/data', () => {
		expect(isSupportedScheme('https://example.com/x.ico')).toBe(true);
		expect(isSupportedScheme('http://example.com/x.ico')).toBe(true);
		expect(isSupportedScheme('data:image/png;base64,xxx')).toBe(true);
	});

	it('rejects browser-internal schemes', () => {
		expect(isSupportedScheme('chrome://favicon/foo')).toBe(false);
		expect(isSupportedScheme('chrome-extension://abc/icon.png')).toBe(false);
		expect(isSupportedScheme('about:blank')).toBe(false);
		expect(isSupportedScheme('moz-extension://abc/icon.png')).toBe(false);
		expect(isSupportedScheme('edge://favicon/foo')).toBe(false);
		expect(isSupportedScheme('brave://newtab')).toBe(false);
		expect(isSupportedScheme('file:///tmp/x')).toBe(false);
	});
});

describe('rankCandidates', () => {
	it('drops candidates with unsupported schemes', () => {
		const ranked = rankCandidates([
			candidate({ href: 'chrome://favicon/foo', order: 0 }),
			candidate({ href: 'https://example.com/icon.png', order: 1 }),
		]);
		expect(ranked).toHaveLength(1);
		expect(ranked[0].href).toBe('https://example.com/icon.png');
	});

	it('prefers SVG over raster regardless of declared size', () => {
		const ranked = rankCandidates([
			candidate({
				href: 'https://example.com/big.png',
				type: 'image/png',
				size: 256,
				order: 0,
			}),
			candidate({
				href: 'https://example.com/icon.svg',
				type: 'image/svg+xml',
				size: Number.POSITIVE_INFINITY,
				order: 1,
			}),
		]);
		expect(ranked[0].href).toBe('https://example.com/icon.svg');
	});

	it('prefers size closer to TARGET_SIZE (64)', () => {
		const ranked = rankCandidates([
			candidate({ href: 'https://example.com/16.png', size: 16, order: 0 }),
			candidate({ href: 'https://example.com/64.png', size: 64, order: 1 }),
			candidate({ href: 'https://example.com/256.png', size: 256, order: 2 }),
		]);
		expect(ranked[0].href).toBe('https://example.com/64.png');
	});

	it('penalizes unknown size against any declared size', () => {
		const ranked = rankCandidates([
			candidate({ href: 'https://example.com/unknown.png', size: 0, order: 0 }),
			candidate({ href: 'https://example.com/16.png', size: 16, order: 1 }),
		]);
		expect(ranked[0].href).toBe('https://example.com/16.png');
	});

	it('breaks size ties by file type (PNG > ICO)', () => {
		const ranked = rankCandidates([
			candidate({
				href: 'https://example.com/icon.ico',
				type: 'image/x-icon',
				size: 0,
				order: 0,
			}),
			candidate({
				href: 'https://example.com/icon.png',
				type: 'image/png',
				size: 0,
				order: 1,
			}),
		]);
		expect(ranked[0].href).toBe('https://example.com/icon.png');
	});

	it('breaks ties on type by source (dom > tab > origin)', () => {
		const ranked = rankCandidates([
			candidate({ source: 'origin', order: 0, href: 'https://example.com/a.png' }),
			candidate({ source: 'tab', order: 1, href: 'https://example.com/b.png' }),
			candidate({ source: 'dom', order: 2, href: 'https://example.com/c.png' }),
		]);
		expect(ranked.map((c) => c.source)).toEqual(['dom', 'tab', 'origin']);
	});

	it('falls through to source order as last tiebreaker', () => {
		const ranked = rankCandidates([
			candidate({ source: 'dom', order: 5, href: 'https://example.com/late.png' }),
			candidate({ source: 'dom', order: 1, href: 'https://example.com/early.png' }),
		]);
		expect(ranked[0].href).toBe('https://example.com/early.png');
	});

	it('does not mutate the input array', () => {
		const input: IconCandidate[] = [
			candidate({ size: 256, order: 0, href: 'https://example.com/a.png' }),
			candidate({ size: 64, order: 1, href: 'https://example.com/b.png' }),
		];
		const snapshot = [...input];
		rankCandidates(input);
		expect(input).toEqual(snapshot);
	});
});

describe('collectIconCandidatesFromLinks', () => {
	it('keeps only icon-related rels', () => {
		const records: IconLinkRecord[] = [
			{ href: 'https://example.com/style.css', rel: 'stylesheet', type: '', sizes: '' },
			{
				href: 'https://example.com/favicon.ico',
				rel: 'icon',
				type: 'image/x-icon',
				sizes: '',
			},
			{
				href: 'https://example.com/apple.png',
				rel: 'apple-touch-icon',
				type: 'image/png',
				sizes: '180x180',
			},
		];
		const candidates = collectIconCandidatesFromLinks(records);
		expect(candidates.map((c) => c.href)).toEqual([
			'https://example.com/favicon.ico',
			'https://example.com/apple.png',
		]);
	});

	it('extracts sizes attribute', () => {
		const records: IconLinkRecord[] = [
			{
				href: 'https://example.com/icon.png',
				rel: 'icon',
				type: 'image/png',
				sizes: '32x32',
			},
		];
		expect(collectIconCandidatesFromLinks(records)[0].size).toBe(32);
	});

	it('preserves DOM order', () => {
		const records: IconLinkRecord[] = [
			{ href: 'https://example.com/1.png', rel: 'icon', type: 'image/png', sizes: '' },
			{
				href: 'https://example.com/2.png',
				rel: 'apple-touch-icon',
				type: 'image/png',
				sizes: '',
			},
			{ href: 'https://example.com/3.png', rel: 'icon', type: 'image/png', sizes: '' },
		];
		const candidates = collectIconCandidatesFromLinks(records);
		expect(candidates.map((c) => c.order)).toEqual([0, 1, 2]);
	});

	it('skips records without href', () => {
		const records: IconLinkRecord[] = [
			{ href: '', rel: 'icon', type: '', sizes: '' },
			{ href: 'https://example.com/x.png', rel: 'icon', type: 'image/png', sizes: '' },
		];
		const candidates = collectIconCandidatesFromLinks(records);
		expect(candidates).toHaveLength(1);
	});
});

describe('tabFaviconCandidate', () => {
	it('returns null when favIconUrl is missing', () => {
		expect(tabFaviconCandidate(undefined, 0)).toBeNull();
		expect(tabFaviconCandidate('', 0)).toBeNull();
	});

	it('builds a tab-source candidate', () => {
		const c = tabFaviconCandidate('https://example.com/favicon.ico', 5);
		expect(c).toEqual({
			href: 'https://example.com/favicon.ico',
			rel: 'icon',
			type: '',
			size: 0,
			source: 'tab',
			order: 5,
		});
	});
});

describe('originFallbackCandidate', () => {
	it('builds /favicon.ico from page URL', () => {
		const c = originFallbackCandidate('https://example.com/some/path?q=1', 7);
		expect(c?.href).toBe('https://example.com/favicon.ico');
		expect(c?.source).toBe('origin');
		expect(c?.order).toBe(7);
	});

	it('returns null when URL is missing', () => {
		expect(originFallbackCandidate(undefined, 0)).toBeNull();
	});

	it('returns null for unparseable URL', () => {
		expect(originFallbackCandidate('not a url', 0)).toBeNull();
	});
});

describe('Wikipedia regression', () => {
	it('picks favicon.ico from a typical Wikipedia link set', () => {
		const records: IconLinkRecord[] = [
			{
				href: 'https://en.wikipedia.org/static/apple-touch/wikipedia.png',
				rel: 'apple-touch-icon',
				type: '',
				sizes: '',
			},
			{
				href: 'https://en.wikipedia.org/static/favicon/wikipedia.ico',
				rel: 'icon',
				type: '',
				sizes: '',
			},
		];
		const candidates = collectIconCandidatesFromLinks(records);
		const fallback = originFallbackCandidate(
			'https://en.wikipedia.org/wiki/Main_Page',
			candidates.length,
		);
		if (fallback) candidates.push(fallback);

		const ranked = rankCandidates(candidates);
		// First choice should be a real Wikipedia icon, not the synthetic origin fallback.
		expect(ranked[0].source).not.toBe('origin');
		expect(ranked[0].href.startsWith('https://en.wikipedia.org/static/')).toBe(true);
	});
});

describe('fetchIconAsBase64', () => {
	let originalFetch: typeof fetch;

	beforeEach(() => {
		originalFetch = global.fetch;
	});

	afterEach(() => {
		global.fetch = originalFetch;
	});

	it('decodes data: URLs without a network call', async () => {
		const fetchSpy = vi.fn();
		global.fetch = fetchSpy as unknown as typeof fetch;

		const out = await fetchIconAsBase64('data:image/png;base64,SGVsbG8=');
		expect(out).toBe('SGVsbG8=');
		expect(fetchSpy).not.toHaveBeenCalled();
	});

	it('throws when the response is not ok', async () => {
		global.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 404,
			statusText: 'Not Found',
		}) as unknown as typeof fetch;
		await expect(fetchIconAsBase64('https://example.com/x.png')).rejects.toThrow(/404/);
	});

	it('rejects HTML responses (e.g. soft-404 pages)', async () => {
		global.fetch = vi.fn().mockResolvedValue({
			ok: true,
			headers: new Headers({ 'content-type': 'text/html' }),
			blob: async () => await Promise.resolve(new Blob(['<html></html>'])),
		}) as unknown as typeof fetch;
		await expect(fetchIconAsBase64('https://example.com/x.png')).rejects.toThrow(
			/content-type/,
		);
	});

	it('accepts image/* responses', async () => {
		const bytes = new Uint8Array([137, 80, 78, 71]);
		global.fetch = vi.fn().mockResolvedValue({
			ok: true,
			headers: new Headers({ 'content-type': 'image/png' }),
			blob: async () => await Promise.resolve(new Blob([bytes], { type: 'image/png' })),
		}) as unknown as typeof fetch;
		const out = await fetchIconAsBase64('https://example.com/x.png');
		expect(out.length).toBeGreaterThan(0);
	});

	it('uses credentials: omit for the favicon fetch', async () => {
		const fetchSpy = vi.fn().mockResolvedValue({
			ok: true,
			headers: new Headers({ 'content-type': 'image/png' }),
			blob: async () => await Promise.resolve(new Blob([new Uint8Array([1, 2, 3])])),
		});
		global.fetch = fetchSpy as unknown as typeof fetch;
		await fetchIconAsBase64('https://example.com/x.png');

		expect(fetchSpy).toHaveBeenCalledTimes(1);
		const init = fetchSpy.mock.calls[0][1];
		expect(init.credentials).toBe('omit');
	});
});

describe('resolveBestCandidate', () => {
	let originalFetch: typeof fetch;

	beforeEach(() => {
		originalFetch = global.fetch;
	});

	afterEach(() => {
		global.fetch = originalFetch;
	});

	it('returns "" when given no candidates', async () => {
		expect(await resolveBestCandidate([])).toBe('');
	});

	it('falls through to the next candidate when one fails', async () => {
		const fetchSpy = vi
			.fn()
			.mockResolvedValueOnce({ ok: false, status: 500, statusText: 'err' })
			.mockResolvedValueOnce({
				ok: true,
				headers: new Headers({ 'content-type': 'image/png' }),
				blob: async () => await Promise.resolve(new Blob([new Uint8Array([1, 2, 3])])),
			});
		global.fetch = fetchSpy as unknown as typeof fetch;

		const result = await resolveBestCandidate([
			candidate({
				href: 'https://example.com/svg.svg',
				type: 'image/svg+xml',
				size: Number.POSITIVE_INFINITY,
				order: 0,
			}),
			candidate({
				href: 'https://example.com/png.png',
				type: 'image/png',
				size: 64,
				order: 1,
			}),
		]);

		expect(result.length).toBeGreaterThan(0);
		expect(fetchSpy).toHaveBeenCalledTimes(2);
	});

	it('deduplicates candidates by href', async () => {
		const fetchSpy = vi.fn().mockResolvedValue({
			ok: true,
			headers: new Headers({ 'content-type': 'image/png' }),
			blob: async () => await Promise.resolve(new Blob([new Uint8Array([1, 2, 3])])),
		});
		global.fetch = fetchSpy as unknown as typeof fetch;

		await resolveBestCandidate([
			candidate({
				href: 'https://example.com/icon.png',
				size: 64,
				source: 'dom',
				order: 0,
			}),
			candidate({
				href: 'https://example.com/icon.png',
				size: 0,
				source: 'tab',
				order: 1,
			}),
		]);

		expect(fetchSpy).toHaveBeenCalledTimes(1);
	});

	it('returns "" when every candidate fails', async () => {
		global.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'err',
		}) as unknown as typeof fetch;
		const result = await resolveBestCandidate([
			candidate({ href: 'https://example.com/a.png' }),
			candidate({ href: 'https://example.com/b.png' }),
		]);
		expect(result).toBe('');
	});
});
