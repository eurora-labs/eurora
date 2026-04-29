/**
 * Domain-keyed UI theme registry.
 *
 * Adding a new theme:
 *   1. Add the theme name to the `ThemeName` union below.
 *   2. Map one or more domains to it in `DOMAIN_THEMES`.
 *   3. Create `packages/ui/src/styles/themes/_<name>.css` that defines
 *      `:root[data-theme='<name>']` and `:root[data-theme='<name>'].dark`
 *      blocks overriding the relevant `--*` color variables.
 *   4. `@import` that file from `packages/ui/src/styles/main.css`.
 *
 * The resolver mirrors the backend's domain canonicalization
 * (`crates/app/euro-activity/src/types.rs::domain_from_url`): lowercase,
 * strip a leading `www.`, then progressively walk up the parent domains so
 * that `m.youtube.com` and `music.youtube.com` both resolve to `youtube`.
 */

export const THEME_NAMES = ['default', 'x', 'youtube', 'google-docs', 'wikipedia'] as const;

export type ThemeName = (typeof THEME_NAMES)[number];

const DOMAIN_THEMES: Readonly<Record<string, ThemeName>> = {
	'x.com': 'x',
	'twitter.com': 'x',
	'youtube.com': 'youtube',
	'docs.google.com': 'google-docs',
	'wikipedia.org': 'wikipedia',
};

export function resolveTheme(domain: string | null | undefined): ThemeName {
	if (!domain) return 'default';

	const host = domain
		.trim()
		.toLowerCase()
		.replace(/^www\./, '');
	if (!host) return 'default';

	const parts = host.split('.');
	for (let i = 0; i < parts.length - 1; i++) {
		const candidate = parts.slice(i).join('.');
		const match = DOMAIN_THEMES[candidate];
		if (match) return match;
	}

	return 'default';
}

/**
 * Pick the theme for a list of currently active context chips. The first chip
 * with a recognized domain wins; otherwise falls back to `default`.
 */
export function resolveThemeFromChips(
	chips: ReadonlyArray<{ domain: string | null }> | null | undefined,
): ThemeName {
	if (!chips) return 'default';
	for (const chip of chips) {
		const theme = resolveTheme(chip.domain);
		if (theme !== 'default') return theme;
	}
	return 'default';
}
