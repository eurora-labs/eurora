import { handleGetPageMetadata } from '../metadata';
import { describe, it, expect, beforeEach } from 'vitest';
import type { PageMetadata } from '../../../bindings';

function setHead(html: string): void {
	document.head.innerHTML = html;
}

function setDocLang(lang: string | null): void {
	if (lang === null) {
		document.documentElement.removeAttribute('lang');
	} else {
		document.documentElement.lang = lang;
	}
}

async function metadata(): Promise<PageMetadata> {
	const response = await handleGetPageMetadata();
	return response.data as PageMetadata;
}

describe('handleGetPageMetadata', () => {
	beforeEach(() => {
		setHead('');
		setDocLang(null);
		document.title = '';
	});

	it('emits a PageMetadata envelope with url/title/host populated', async () => {
		document.title = 'Example';
		const response = await handleGetPageMetadata();
		expect(response.kind).toBe('PageMetadata');
		const data = response.data as PageMetadata;
		expect(data.title).toBe('Example');
		expect(data.url).toBe(window.location.href);
		expect(data.host).toBe(window.location.host);
	});

	it('resolves language from <html lang> first', async () => {
		setDocLang('en-GB');
		setHead('<meta http-equiv="content-language" content="fr">');
		const data = await metadata();
		expect(data.language).toBe('en-GB');
	});

	it('falls back to <meta http-equiv="content-language"> when <html lang> is absent', async () => {
		setHead('<meta http-equiv="content-language" content="fr">');
		const data = await metadata();
		expect(data.language).toBe('fr');
	});

	it('returns null language when neither source is set', async () => {
		const data = await metadata();
		expect(data.language).toBeNull();
	});

	it('parses <meta name="description">', async () => {
		setHead('<meta name="description" content="hello world">');
		const data = await metadata();
		expect(data.description).toBe('hello world');
	});

	it('collects OpenGraph tags into an og map keyed by suffix', async () => {
		setHead(`
			<meta property="og:title" content="OG Title">
			<meta property="og:site_name" content="Example Inc">
			<meta property="og:image" content="https://example.com/a.png">
		`);
		const data = await metadata();
		expect(data.og).toEqual({
			title: 'OG Title',
			site_name: 'Example Inc',
			image: 'https://example.com/a.png',
		});
	});

	it('keeps document order for duplicate og properties', async () => {
		setHead(`
			<meta property="og:title" content="First">
			<meta property="og:title" content="Second">
		`);
		const data = await metadata();
		expect(data.og['title']).toBe('First');
	});

	it('reports viewport metrics derived from window/scrollHeight', async () => {
		const data = await metadata();
		expect(data.viewport.inner_width).toBe(window.innerWidth);
		expect(data.viewport.inner_height).toBe(window.innerHeight);
		expect(data.viewport.scroll_x).toBe(0);
		expect(data.viewport.scroll_y).toBe(0);
		expect(typeof data.viewport.document_height).toBe('number');
	});
});
