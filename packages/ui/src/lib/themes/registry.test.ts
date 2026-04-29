import { resolveTheme, resolveThemeFromChips } from '$lib/themes/registry.js';
import { describe, expect, it } from 'vitest';

describe('resolveTheme', () => {
	it('returns default for null/undefined/empty domains', () => {
		expect(resolveTheme(null)).toBe('default');
		expect(resolveTheme(undefined)).toBe('default');
		expect(resolveTheme('')).toBe('default');
		expect(resolveTheme('   ')).toBe('default');
	});

	it('matches exact registered domains', () => {
		expect(resolveTheme('x.com')).toBe('x');
		expect(resolveTheme('twitter.com')).toBe('x');
		expect(resolveTheme('youtube.com')).toBe('youtube');
		expect(resolveTheme('docs.google.com')).toBe('google-docs');
		expect(resolveTheme('wikipedia.org')).toBe('wikipedia');
	});

	it('strips a leading www and matches', () => {
		expect(resolveTheme('www.x.com')).toBe('x');
		expect(resolveTheme('www.youtube.com')).toBe('youtube');
		expect(resolveTheme('www.wikipedia.org')).toBe('wikipedia');
	});

	it('matches subdomains by walking up parent domains', () => {
		expect(resolveTheme('m.youtube.com')).toBe('youtube');
		expect(resolveTheme('music.youtube.com')).toBe('youtube');
		expect(resolveTheme('en.wikipedia.org')).toBe('wikipedia');
		expect(resolveTheme('de.m.wikipedia.org')).toBe('wikipedia');
	});

	it('does not promote sibling domains under google.com to google-docs', () => {
		// docs.google.com is registered, but plain google.com / mail.google.com are not.
		expect(resolveTheme('google.com')).toBe('default');
		expect(resolveTheme('mail.google.com')).toBe('default');
	});

	it('is case insensitive', () => {
		expect(resolveTheme('YouTube.com')).toBe('youtube');
		expect(resolveTheme('Docs.Google.COM')).toBe('google-docs');
	});

	it('returns default for unknown domains', () => {
		expect(resolveTheme('example.com')).toBe('default');
		expect(resolveTheme('localhost')).toBe('default');
	});
});

describe('resolveThemeFromChips', () => {
	it('returns default for nullish or empty chip lists', () => {
		expect(resolveThemeFromChips(null)).toBe('default');
		expect(resolveThemeFromChips(undefined)).toBe('default');
		expect(resolveThemeFromChips([])).toBe('default');
	});

	it('returns the first chip with a recognized domain', () => {
		expect(
			resolveThemeFromChips([
				{ domain: null },
				{ domain: 'youtube.com' },
				{ domain: 'x.com' },
			]),
		).toBe('youtube');
	});

	it('skips unknown domains and falls back to default', () => {
		expect(resolveThemeFromChips([{ domain: 'example.com' }, { domain: null }])).toBe(
			'default',
		);
	});
});
