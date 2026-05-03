import { countWords } from '$lib/word/extract';
import { describe, expect, it } from 'vitest';

describe('countWords', () => {
	it('returns 0 for empty and whitespace-only strings', () => {
		expect(countWords('')).toBe(0);
		expect(countWords('   \n\t')).toBe(0);
	});

	it('counts whitespace-separated tokens', () => {
		expect(countWords('hello world')).toBe(2);
		expect(countWords('  the  quick   brown fox  ')).toBe(4);
	});

	it('treats newlines and tabs as separators', () => {
		expect(countWords('one\ntwo\tthree four')).toBe(4);
	});
});
