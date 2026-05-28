import { normalizeSelectionForContext, readSelectionForContext } from '../selection';
import { describe, it, expect, afterEach } from 'vitest';

describe('normalizeSelectionForContext', () => {
	it('returns empty string for empty input', () => {
		expect(normalizeSelectionForContext('')).toBe('');
	});

	it('returns empty string for whitespace-only input', () => {
		expect(normalizeSelectionForContext('   \n\t  ')).toBe('');
	});

	it('trims leading and trailing whitespace', () => {
		expect(normalizeSelectionForContext('  hello world  ')).toBe('hello world');
	});

	it('collapses internal whitespace runs to single spaces', () => {
		expect(normalizeSelectionForContext('hello\n\n  world\t!')).toBe('hello world !');
	});

	it('returns the full string when under the length cap', () => {
		const text = 'a'.repeat(100);
		expect(normalizeSelectionForContext(text, 200)).toBe(text);
	});

	it('returns the full string when exactly at the length cap', () => {
		const text = 'a'.repeat(50);
		expect(normalizeSelectionForContext(text, 50)).toBe(text);
	});

	it('truncates with ellipsis and total length suffix when over the cap', () => {
		const text = 'a'.repeat(120);
		const result = normalizeSelectionForContext(text, 50);
		expect(result).toBe(`${'a'.repeat(50)}… (120 characters total)`);
	});

	it('measures length after whitespace collapse, not before', () => {
		const text = `${'a'.repeat(30)}     ${'b'.repeat(30)}`;
		const collapsed = `${'a'.repeat(30)} ${'b'.repeat(30)}`;
		expect(normalizeSelectionForContext(text, 200)).toBe(collapsed);
	});

	it('honors a custom maxLen', () => {
		expect(normalizeSelectionForContext('abcdefghij', 4)).toBe('abcd… (10 characters total)');
	});
});

describe('readSelectionForContext', () => {
	const originalGetSelection = window.getSelection.bind(window);

	afterEach(() => {
		window.getSelection = originalGetSelection;
	});

	function mockSelection(text: string | null): void {
		window.getSelection = () =>
			text === null
				? null
				: ({
						toString: () => text,
					} as unknown as Selection);
	}

	it('returns empty string when there is no selection', () => {
		mockSelection(null);
		expect(readSelectionForContext()).toBe('');
	});

	it('returns empty string for a whitespace-only selection', () => {
		mockSelection('   \n  ');
		expect(readSelectionForContext()).toBe('');
	});

	it('returns the normalized selection when there is one', () => {
		mockSelection('  hello\n  world  ');
		expect(readSelectionForContext()).toBe('hello world');
	});

	it('truncates long selections using the default cap', () => {
		mockSelection('x'.repeat(800));
		const result = readSelectionForContext();
		expect(result.startsWith('x'.repeat(500))).toBe(true);
		expect(result.endsWith(' (800 characters total)')).toBe(true);
	});
});
