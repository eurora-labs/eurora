/// Shared Google Docs helpers used by the docs.google.com tools. Detects
/// which sub-product (`document` or `spreadsheets`) the page belongs to
/// and extracts the resource id + title needed to address it. Tools
/// themselves stay self-contained otherwise.

export type GoogleDocKind = 'document' | 'spreadsheets';

export function detectDocKind(): GoogleDocKind | null {
	const path = window.location.pathname;
	if (path.startsWith('/document/')) return 'document';
	if (path.startsWith('/spreadsheets/')) return 'spreadsheets';
	return null;
}

/// Resolve the current doc kind or throw. Used by tools that only make
/// sense on an actual document or spreadsheet; the thrown error surfaces
/// to the bridge as `{ err: { kind: 'adapter', message } }`.
export function requireDocKind(): GoogleDocKind {
	const kind = detectDocKind();
	if (kind === null) {
		throw new Error('not on a Google Docs document');
	}
	return kind;
}

/// Title resolution ladder: the live editor's title input (most
/// reliable while the doc is open), the read-only title widget (shows
/// while the title input is still loading), then `document.title` with
/// the trailing product suffix stripped.
export function getDocTitle(): string {
	const titleInput = document.querySelector<HTMLInputElement>('.docs-title-input');
	if (titleInput?.value) return titleInput.value;
	const widgetText = document.querySelector('.docs-title-widget')?.textContent?.trim();
	if (widgetText) return widgetText;
	return document.title.replace(/ - Google (Docs|Sheets)$/, '');
}

export function getResourceId(kind: GoogleDocKind): string {
	const match = window.location.pathname.match(new RegExp(`/${kind}/d/([a-zA-Z0-9_-]+)`));
	if (!match) {
		throw new Error(`could not extract resource id from ${window.location.pathname}`);
	}
	return match[1];
}

export function siteName(kind: GoogleDocKind): 'Google Docs' | 'Google Sheets' {
	return kind === 'spreadsheets' ? 'Google Sheets' : 'Google Docs';
}
