import { handleInsertText, SAFETY_VIOLATION } from '../insert-text';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { InsertTextResult } from '../../../bindings';
import type { BrowserObj } from '../../watchers/watcher';

function args(overrides: Partial<BrowserObj> & { field_id: string; text: string }): BrowserObj {
	return { type: 'INSERT_TEXT', ...overrides };
}

describe('handleInsertText', () => {
	beforeEach(() => {
		document.body.innerHTML = '';
	});

	it('writes text into a text input and emits a single bubbling input event', async () => {
		document.body.innerHTML = '<input id="user" type="text" value="">';
		const input = document.getElementById('user') as HTMLInputElement;
		const inputSpy = vi.fn();
		const bannedSpy = vi.fn();
		input.addEventListener('input', inputSpy);
		for (const evt of ['change', 'keydown', 'keyup', 'keypress', 'focus', 'blur']) {
			input.addEventListener(evt, bannedSpy);
		}
		// `submit` fires on the form, not on the input — listen on the parent.
		document.body.addEventListener('submit', bannedSpy);

		const response = await handleInsertText(args({ field_id: '#user', text: 'hello' }));
		expect(response.kind).toBe('InsertTextResult');
		const result = response.data as InsertTextResult;
		expect(result.previous_value).toBe('');
		expect(result.new_value).toBe('hello');
		expect(input.value).toBe('hello');
		expect(inputSpy).toHaveBeenCalledTimes(1);
		expect(bannedSpy).not.toHaveBeenCalled();
	});

	it('appends to the existing value by default', async () => {
		document.body.innerHTML = '<input id="q" type="search" value="cats">';
		const response = await handleInsertText(args({ field_id: '#q', text: ' and dogs' }));
		const result = response.data as InsertTextResult;
		expect(result.new_value).toBe('cats and dogs');
		expect((document.getElementById('q') as HTMLInputElement).value).toBe('cats and dogs');
	});

	it('replaces the value when replace=true', async () => {
		document.body.innerHTML = '<input id="q" type="text" value="old">';
		const response = await handleInsertText(
			args({ field_id: '#q', text: 'new', replace: true }),
		);
		const result = response.data as InsertTextResult;
		expect(result.previous_value).toBe('old');
		expect(result.new_value).toBe('new');
	});

	it('writes through the React-controlled value setter idiom', async () => {
		document.body.innerHTML = '<input id="r" type="text" value="">';
		const prototypeSetter = Object.getOwnPropertyDescriptor(
			HTMLInputElement.prototype,
			'value',
		)!.set!;
		const setterSpy = vi.fn(prototypeSetter);
		Object.defineProperty(HTMLInputElement.prototype, 'value', {
			configurable: true,
			set: setterSpy,
			get() {
				return (this as any)['__v'] ?? '';
			},
		});
		try {
			await handleInsertText(args({ field_id: '#r', text: 'wired' }));
			expect(setterSpy).toHaveBeenCalled();
		} finally {
			Object.defineProperty(HTMLInputElement.prototype, 'value', {
				configurable: true,
				set: prototypeSetter,
				get: Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.get!,
			});
		}
	});

	it('writes into <textarea> via its prototype value setter', async () => {
		document.body.innerHTML = '<textarea id="t"></textarea>';
		const ta = document.getElementById('t') as HTMLTextAreaElement;
		const inputSpy = vi.fn();
		ta.addEventListener('input', inputSpy);
		await handleInsertText(args({ field_id: '#t', text: 'line' }));
		expect(ta.value).toBe('line');
		expect(inputSpy).toHaveBeenCalledTimes(1);
	});

	it('writes into [contenteditable] via textContent', async () => {
		document.body.innerHTML = '<div id="ce" contenteditable="true"></div>';
		const ce = document.getElementById('ce')!;
		await handleInsertText(args({ field_id: '#ce', text: 'edited' }));
		expect(ce.textContent).toBe('edited');
	});

	it('rejects a field_id that resolves to zero elements', async () => {
		const response = await handleInsertText(args({ field_id: '#nope', text: 'x' }));
		expect(response.kind).toBe('Error');
		expect((response as unknown as { code: string }).code).toBe(SAFETY_VIOLATION);
		expect(String(response.data)).toMatch(/matched zero/);
	});

	it('rejects a field_id that resolves to multiple elements', async () => {
		document.body.innerHTML = '<input class="x" type="text"><input class="x" type="text">';
		const response = await handleInsertText(args({ field_id: '.x', text: 'x' }));
		expect(response.kind).toBe('Error');
		expect((response as unknown as { code: string }).code).toBe(SAFETY_VIOLATION);
		expect(String(response.data)).toMatch(/matched 2/);
	});

	it('rejects writes into password inputs', async () => {
		document.body.innerHTML = '<input id="pw" type="password">';
		const response = await handleInsertText(args({ field_id: '#pw', text: 'leak' }));
		expect(response.kind).toBe('Error');
		expect((response as unknown as { code: string }).code).toBe(SAFETY_VIOLATION);
	});

	it('rejects writes into file/checkbox/radio/submit inputs', async () => {
		for (const type of ['file', 'checkbox', 'radio', 'submit']) {
			document.body.innerHTML = `<input id="x" type="${type}">`;
			const response = await handleInsertText(args({ field_id: '#x', text: 'x' }));
			expect(response.kind).toBe('Error');
			expect((response as unknown as { code: string }).code).toBe(SAFETY_VIOLATION);
		}
	});

	it('rejects writes into disabled and readonly fields', async () => {
		document.body.innerHTML = '<input id="d" type="text" disabled>';
		let response = await handleInsertText(args({ field_id: '#d', text: 'x' }));
		expect((response as unknown as { code: string }).code).toBe(SAFETY_VIOLATION);

		document.body.innerHTML = '<input id="r" type="text" readonly>';
		response = await handleInsertText(args({ field_id: '#r', text: 'x' }));
		expect((response as unknown as { code: string }).code).toBe(SAFETY_VIOLATION);
	});

	it('rejects malformed args (missing text)', async () => {
		document.body.innerHTML = '<input id="x" type="text">';
		const response = await handleInsertText({ type: 'INSERT_TEXT', field_id: '#x' });
		expect((response as unknown as { code: string }).code).toBe(SAFETY_VIOLATION);
	});

	it('rejects an invalid CSS selector', async () => {
		const response = await handleInsertText(args({ field_id: '!!', text: 'x' }));
		expect((response as unknown as { code: string }).code).toBe(SAFETY_VIOLATION);
	});
});
