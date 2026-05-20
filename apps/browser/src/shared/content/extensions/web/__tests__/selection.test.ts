import { handleGetSelectedText } from '../selection';
import { describe, it, expect, beforeEach } from 'vitest';
import type { SelectedText } from '../../../bindings';

async function selectedText(): Promise<SelectedText> {
	const response = await handleGetSelectedText();
	return response.data as SelectedText;
}

describe('handleGetSelectedText', () => {
	beforeEach(() => {
		document.body.innerHTML = '';
		window.getSelection()?.removeAllRanges();
	});

	it('returns an empty payload when nothing is selected', async () => {
		const data = await selectedText();
		expect(data.text).toBe('');
		expect(data.anchor_xpath).toBeNull();
		expect(data.focus_xpath).toBeNull();
	});

	it('emits a SelectedText envelope', async () => {
		const response = await handleGetSelectedText();
		expect(response.kind).toBe('SelectedText');
	});

	it('captures the selection text and anchor/focus XPaths', async () => {
		document.body.innerHTML = '<p>hello world</p>';
		const p = document.querySelector('p')!;
		const range = document.createRange();
		range.selectNodeContents(p);
		const selection = window.getSelection()!;
		selection.removeAllRanges();
		selection.addRange(range);

		const data = await selectedText();
		expect(data.text).toBe('hello world');
		expect(data.anchor_xpath).toContain('p[');
		expect(data.focus_xpath).toContain('p[');
	});
});
