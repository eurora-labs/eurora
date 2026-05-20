import { handleListFormInputs } from '../form-inputs';
import { describe, it, expect, beforeEach } from 'vitest';
import type { FormInputsList } from '../../../bindings';
import type { BrowserObj } from '../../watchers/watcher';

function args(overrides: Partial<BrowserObj> = {}): BrowserObj {
	return { type: 'LIST_FORM_INPUTS', ...overrides };
}

async function call(overrides: Partial<BrowserObj> = {}): Promise<FormInputsList> {
	const response = await handleListFormInputs(args(overrides));
	return response.data as FormInputsList;
}

describe('handleListFormInputs', () => {
	beforeEach(() => {
		document.body.innerHTML = '';
	});

	it('emits a FormInputsList envelope', async () => {
		document.body.innerHTML = '<input type="text">';
		const response = await handleListFormInputs(args());
		expect(response.kind).toBe('FormInputsList');
	});

	it('excludes password/file/hidden/checkbox/radio/submit/image inputs', async () => {
		document.body.innerHTML = `
			<form>
				<input type="text" name="user">
				<input type="password" name="password">
				<input type="file" name="file">
				<input type="hidden" name="csrf" value="x">
				<input type="checkbox" name="terms">
				<input type="radio" name="plan">
				<input type="submit" value="Go">
				<input type="image" src="">
			</form>
		`;
		const result = await call();
		expect(result.inputs).toHaveLength(1);
		expect(result.inputs[0].kind).toBe('text');
		// `total` reflects allowlisted, in-scope fields only.
		expect(result.total).toBe(1);
	});

	it('excludes disabled and readonly fields', async () => {
		document.body.innerHTML = `
			<input type="text" name="ok">
			<input type="text" name="dis" disabled>
			<input type="text" name="ro" readonly>
		`;
		const result = await call();
		expect(result.inputs).toHaveLength(1);
		expect(result.inputs[0].label).toBeNull();
	});

	it('includes textarea and contenteditable kinds', async () => {
		document.body.innerHTML = `
			<textarea aria-label="Body"></textarea>
			<div contenteditable="true" aria-label="Editor">draft</div>
		`;
		const result = await call();
		const kinds = result.inputs.map((i) => i.kind).sort();
		expect(kinds).toEqual(['content_editable', 'textarea']);
	});

	it('resolves label via <label for>', async () => {
		document.body.innerHTML = `
			<label for="email">Email</label>
			<input id="email" type="email">
		`;
		const result = await call();
		expect(result.inputs[0].label).toBe('Email');
	});

	it('resolves label via wrapping <label>', async () => {
		document.body.innerHTML = `
			<label>Search <input type="search"></label>
		`;
		const result = await call();
		expect(result.inputs[0].label).toContain('Search');
	});

	it('resolves label via aria-labelledby', async () => {
		document.body.innerHTML = `
			<span id="lbl">Username</span>
			<input type="text" aria-labelledby="lbl">
		`;
		const result = await call();
		expect(result.inputs[0].label).toBe('Username');
	});

	it('captures placeholder, value, and required', async () => {
		document.body.innerHTML = `
			<input type="text" placeholder="search…" value="cats" required>
		`;
		const result = await call();
		expect(result.inputs[0].placeholder).toBe('search…');
		expect(result.inputs[0].value).toBe('cats');
		expect(result.inputs[0].required).toBe(true);
	});
});
