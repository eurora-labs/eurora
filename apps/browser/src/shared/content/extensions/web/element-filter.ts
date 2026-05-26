/// Categorisation of writable text fields surfaced by
/// [`writableFieldKind`]. The set deliberately excludes password / file /
/// hidden / submit-style inputs (those are denied by the safety
/// contract regardless of dispatch path) and non-text input types
/// (checkbox, radio, range) that don't accept text.
export type FormInputKind =
	| 'text'
	| 'search'
	| 'email'
	| 'url'
	| 'tel'
	| 'number'
	| 'textarea'
	| 'content_editable';

/**
 * Selectors elided from every `query_selector` result regardless of what
 * the caller asked for. The model can't see these elements; if the user
 * wants the LLM to know a sensitive value, they paste it into chat.
 *
 * The denylist covers two categories:
 *
 *   1. Form controls that don't have a model-meaningful textual value
 *      (`hidden`/`file`/`submit`/`image`/`reset`/`checkbox`/`radio`) or
 *      that carry user-secret values the model must never observe
 *      (`password`).
 *   2. Page-state tokens the model could exfiltrate or use to forge
 *      authenticated requests if it leaked into chat history
 *      (`<meta>` carrying CSRF/XSRF/authenticity tokens).
 *
 * Visibility-based filtering (`[hidden]`, `aria-hidden="true"`,
 * computed `display:none`/`visibility:hidden`) is applied at the call
 * site via `isVisible` so that the test fixtures don't have to inject
 * full stylesheets to exercise hidden-element paths.
 */
export const ELEMENT_SAFETY_DENYLIST: readonly string[] = Object.freeze([
	'input[type="hidden"]',
	'input[type="password"]',
	'input[type="file"]',
	'input[type="submit"]',
	'input[type="image"]',
	'input[type="reset"]',
	'input[type="checkbox"]',
	'input[type="radio"]',
	'meta[name="csrf-token"]',
	'meta[name="csrf_token"]',
	'meta[name="xsrf-token"]',
	'meta[name="xsrf_token"]',
	'meta[name="authenticity_token"]',
	// Heuristic: anything with "csrf" or "xsrf" anywhere in the name.
	'meta[name*="csrf" i]',
	'meta[name*="xsrf" i]',
]);

const DENYLIST_SELECTOR = ELEMENT_SAFETY_DENYLIST.join(',');

const WRITABLE_INPUT_TYPES: ReadonlyMap<string, FormInputKind> = new Map([
	['text', 'text'],
	['search', 'search'],
	['email', 'email'],
	['url', 'url'],
	['tel', 'tel'],
	['number', 'number'],
]);

/**
 * `true` when `el` matches the [`ELEMENT_SAFETY_DENYLIST`]. Cheaper than
 * checking each denylist entry individually because `matches` short-
 * circuits on the first hit.
 */
export function isDenylisted(el: Element): boolean {
	try {
		return el.matches(DENYLIST_SELECTOR);
	} catch {
		// `matches` throws on selector syntax errors — never seen in
		// practice with the static denylist, but defensive.
		return false;
	}
}

/**
 * `true` when `el` is laid out and visible to the user. The heuristic
 * mirrors what assistive technologies treat as visible: an element is
 * visible iff
 *
 *   - it doesn't carry the `hidden` attribute,
 *   - it doesn't carry `aria-hidden="true"`,
 *   - its computed `display` is not `none`,
 *   - its computed `visibility` is not `hidden` or `collapse`.
 *
 * Bounds-based visibility (zero-size, off-screen) is **not** part of
 * this check — sites legitimately hide focusable controls under sticky
 * headers, and the model shouldn't lose access to them.
 */
export function isVisible(el: Element): boolean {
	if (el.hasAttribute('hidden')) {
		return false;
	}
	if (el.getAttribute('aria-hidden') === 'true') {
		return false;
	}
	const win = el.ownerDocument?.defaultView;
	if (!win) {
		// Detached element. Treat as not visible.
		return false;
	}
	const style = win.getComputedStyle(el);
	if (style.display === 'none') {
		return false;
	}
	if (style.visibility === 'hidden' || style.visibility === 'collapse') {
		return false;
	}
	return true;
}

/**
 * Resolve an element to its [`FormInputKind`] when it is a writable text
 * field, or `null` otherwise. Disabled and read-only fields are treated
 * as non-writable.
 *
 * This is the inverse of the input-type denylist: it implements the
 * positive allowlist `insert_text` enforces.
 */
export function writableFieldKind(el: Element): FormInputKind | null {
	if (el instanceof HTMLInputElement) {
		if (el.disabled || el.readOnly) {
			return null;
		}
		// `<input>` with no explicit type defaults to `text`.
		const type = (el.getAttribute('type') ?? 'text').toLowerCase();
		return WRITABLE_INPUT_TYPES.get(type) ?? null;
	}
	if (el instanceof HTMLTextAreaElement) {
		if (el.disabled || el.readOnly) {
			return null;
		}
		return 'textarea';
	}
	if (el instanceof HTMLElement && isContentEditable(el)) {
		if (el.getAttribute('aria-readonly') === 'true') {
			return null;
		}
		return 'content_editable';
	}
	return null;
}

/**
 * Cross-engine `isContentEditable` check. jsdom doesn't implement the
 * inherited-value computation that the spec requires, so we read the
 * attribute directly and honour the documented "inherit" / "true" /
 * empty-string values.
 */
function isContentEditable(el: HTMLElement): boolean {
	const attr = el.getAttribute('contenteditable');
	if (attr === null) {
		return false;
	}
	const normalized = attr.toLowerCase();
	return normalized === '' || normalized === 'true' || normalized === 'plaintext-only';
}
