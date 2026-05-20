import { handleListLinks } from '../links';
import { describe, it, expect, beforeEach } from 'vitest';
import type { LinksList } from '../../../bindings';
import type { BrowserObj } from '../../watchers/watcher';

function args(overrides: Partial<BrowserObj> = {}): BrowserObj {
	return { type: 'LIST_LINKS', ...overrides };
}

async function call(overrides: Partial<BrowserObj> = {}): Promise<LinksList> {
	const response = await handleListLinks(args(overrides));
	return response.data as LinksList;
}

describe('handleListLinks', () => {
	beforeEach(() => {
		document.body.innerHTML = '';
	});

	it('emits a LinksList envelope', async () => {
		document.body.innerHTML = '<a href="https://example.com">x</a>';
		const response = await handleListLinks(args());
		expect(response.kind).toBe('LinksList');
	});

	it('resolves absolute URLs and skips non-http schemes / hash links', async () => {
		document.body.innerHTML = `
			<a href="https://example.com/a">good</a>
			<a href="/relative">relative</a>
			<a href="#anchor">hash</a>
			<a href="mailto:x@y">mail</a>
			<a href="javascript:void(0)">js</a>
		`;
		const result = await call();
		const urls = result.links.map((l) => l.url);
		expect(urls).toContain('https://example.com/a');
		expect(urls.some((u) => u.endsWith('/relative'))).toBe(true);
		expect(urls.find((u) => u.includes('mailto'))).toBeUndefined();
		expect(urls.find((u) => u.includes('javascript'))).toBeUndefined();
		expect(urls.find((u) => u.endsWith('#anchor'))).toBeUndefined();
	});

	it('uses aria-label, text, or title for the label, in order', async () => {
		document.body.innerHTML = `
			<a href="https://x.example/a" aria-label="Aria">x</a>
			<a href="https://x.example/b">Plain text</a>
			<a href="https://x.example/c" title="Title only"></a>
		`;
		const result = await call();
		const labels = result.links.map((l) => l.label);
		expect(labels).toContain('Aria');
		expect(labels).toContain('Plain text');
		expect(labels).toContain('Title only');
	});

	it('honours `limit` and reports total separately', async () => {
		const html = Array.from(
			{ length: 5 },
			(_, i) => `<a href="https://x.example/${i}">${i}</a>`,
		).join('');
		document.body.innerHTML = html;
		const result = await call({ limit: 2 });
		expect(result.links).toHaveLength(2);
		expect(result.total).toBe(5);
	});
});
