/// Default character cap for selection snippets embedded in a context
/// string. Picked to be long enough for a paragraph but short enough that
/// a user who has selected the entire article doesn't blow the
/// system-message budget. Tunable per call site if a different surface
/// has stricter or looser needs.
const DEFAULT_CONTEXT_LIMIT = 500;

/// Collapse whitespace runs, trim, and cap at `maxLen` characters. Returns
/// `''` for whitespace-only input so callers can use truthiness as the
/// "something meaningful is selected" predicate.
///
/// Kept separate from the DOM read so the truncation behavior can be
/// unit-tested with literal strings — no fake `Selection` plumbing
/// required.
export function normalizeSelectionForContext(
	raw: string,
	maxLen: number = DEFAULT_CONTEXT_LIMIT,
): string {
	const collapsed = raw.replace(/\s+/g, ' ').trim();
	if (!collapsed) return '';
	if (collapsed.length <= maxLen) return collapsed;
	return `${collapsed.slice(0, maxLen)}… (${collapsed.length} characters total)`;
}

/// Read the user's current text selection and normalize it for inclusion
/// in a per-site context summary. Returns `''` when nothing meaningful is
/// selected (no selection at all, or a whitespace-only one).
///
/// This is the canonical reader for "is the user pointing at something
/// right now?" context clauses. Tools that need exact, untruncated
/// selection text (e.g. `web_get_selected_text`) read the raw `Selection`
/// themselves — they have different fidelity requirements.
export function readSelectionForContext(maxLen: number = DEFAULT_CONTEXT_LIMIT): string {
	const raw = window.getSelection()?.toString() ?? '';
	return normalizeSelectionForContext(raw, maxLen);
}
