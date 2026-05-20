import { handleQuerySelector } from '../query-selector';
import { describe, it, expect, beforeEach } from 'vitest';
import type { QuerySelectorResult } from '../../../bindings';
import type { BrowserObj } from '../../watchers/watcher';

function args(overrides: Partial<BrowserObj> & { selector: string }): BrowserObj {
	return { type: 'QUERY_SELECTOR', ...overrides };
}

async function call(
	overrides: Partial<BrowserObj> & { selector: string },
): Promise<QuerySelectorResult> {
	const response = await handleQuerySelector(args(overrides));
	return response.data as QuerySelectorResult;
}

describe('handleQuerySelector', () => {
	beforeEach(() => {
		document.body.innerHTML = '';
	});

	it('returns nodes carrying only selector_path when include is empty', async () => {
		document.body.innerHTML = '<p>Hello</p>';
		const result = await call({ selector: 'p' });
		expect(result.matches).toHaveLength(1);
		expect(result.matches[0].text).toBeNull();
		expect(result.matches[0].html).toBeNull();
		expect(result.matches[0].attributes).toBeNull();
		expect(result.matches[0].bounds).toBeNull();
		expect(typeof result.matches[0].selector_path).toBe('string');
	});

	it('populates text when include=["text"]', async () => {
		document.body.innerHTML = '<p>Hello</p>';
		const result = await call({ selector: 'p', include: ['text'] });
		expect(result.matches[0].text).toBe('Hello');
	});

	it('populates html when include=["html"]', async () => {
		document.body.innerHTML = '<p class="a">Hi</p>';
		const result = await call({ selector: 'p', include: ['html'] });
		expect(result.matches[0].html).toBe('<p class="a">Hi</p>');
	});

	it('populates attributes when include=["attributes"]', async () => {
		document.body.innerHTML = '<a class="x" data-test="y" href="/q">link</a>';
		const result = await call({ selector: 'a', include: ['attributes'] });
		expect(result.matches[0].attributes).toEqual({ class: 'x', 'data-test': 'y', href: '/q' });
	});

	it('elides denylisted elements while still counting them in total_match_count', async () => {
		document.body.innerHTML = `
			<form>
				<input type="text" name="user">
				<input type="password" name="password">
				<input type="hidden" name="csrf">
			</form>
		`;
		const result = await call({ selector: 'input' });
		// 3 raw matches; only the text input survives the denylist.
		expect(result.total_match_count).toBe(3);
		expect(result.matches).toHaveLength(1);
	});

	it('elides CSRF-shaped meta tags', async () => {
		document.head.innerHTML = '<meta name="csrf-token" content="secret">';
		const result = await call({ selector: 'meta' });
		expect(result.total_match_count).toBeGreaterThanOrEqual(1);
		expect(result.matches).toHaveLength(0);
		document.head.innerHTML = '';
	});

	it('caps the result at `limit` and sets truncated=true', async () => {
		const items = Array.from({ length: 4 }, (_, i) => `<p>p${i}</p>`).join('');
		document.body.innerHTML = items;
		const result = await call({ selector: 'p', limit: 2 });
		expect(result.matches).toHaveLength(2);
		expect(result.total_match_count).toBe(4);
		expect(result.truncated).toBe(true);
	});

	it('truncates oversized text and sets truncated=true', async () => {
		const big = 'x'.repeat(9 * 1024);
		document.body.innerHTML = `<p>${big}</p>`;
		const result = await call({ selector: 'p', include: ['text'] });
		expect(result.matches[0].text!.length).toBeLessThanOrEqual(8 * 1024);
		expect(result.truncated).toBe(true);
	});

	it('rejects an empty selector', async () => {
		await expect(handleQuerySelector(args({ selector: '   ' }))).rejects.toThrow(/non-empty/);
	});

	it('rejects an invalid CSS selector with a structured message', async () => {
		await expect(handleQuerySelector(args({ selector: '!!' }))).rejects.toThrow(
			/not a valid CSS/,
		);
	});
});
