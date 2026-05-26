/**
 * Build a stable CSS selector for an element so that subsequent web-tool
 * calls (`query_selector`, `insert_text`, …) can resolve the same node.
 *
 * Stability ladder, from most to least preferred:
 *
 *   1. `#id` — when the element carries an `id` that is a syntactically
 *      valid CSS identifier (kept simple here; the strict CSS.escape
 *      grammar is overkill for the LLM-facing surface).
 *   2. `[data-testid=…]`, `[data-test-id=…]`, `[name=…]` — author-stable
 *      hooks emitted by component libraries and test harnesses.
 *   3. A short chain rooted at the nearest *semantic landmark* ancestor
 *      (`main`, `nav`, `header`, `footer`, `aside`, `[role=main]`, …),
 *      with each step disambiguated by `:nth-of-type(n)`.
 *   4. A full `:nth-of-type(n)` chain rooted at `<body>`.
 *
 * The output is always a selector that, when fed back through
 * `document.querySelectorAll(...)`, resolves to exactly the element the
 * caller passed in. The `__tests__/selector-path.test.ts` suite enforces
 * that round-trip property.
 */

const SIMPLE_ID = /^[A-Za-z][\w-]*$/;

const STABLE_ATTRS = ['data-testid', 'data-test-id', 'name'] as const;

const LANDMARK_SELECTORS = [
	'main',
	'nav',
	'header',
	'footer',
	'aside',
	'article',
	'section[aria-label]',
	'section[aria-labelledby]',
	'[role="main"]',
	'[role="navigation"]',
	'[role="banner"]',
	'[role="contentinfo"]',
	'[role="complementary"]',
] as const;

export function buildSelectorPath(el: Element): string {
	if (!(el instanceof Element)) {
		throw new Error('buildSelectorPath requires an Element');
	}

	if (el.id && SIMPLE_ID.test(el.id)) {
		const candidate = `#${el.id}`;
		if (resolvesUniquely(el, candidate)) {
			return candidate;
		}
	}

	for (const attr of STABLE_ATTRS) {
		const value = el.getAttribute(attr);
		if (value && value.length > 0 && value.length < 128) {
			const candidate = `${el.localName}[${attr}="${cssEscapeAttr(value)}"]`;
			if (resolvesUniquely(el, candidate)) {
				return candidate;
			}
		}
	}

	// If this element is itself a uniquely-tagged landmark, that tag
	// alone is the best selector — `html > main:nth-of-type(1)` would
	// round-trip but is uglier and less stable across re-layouts.
	if (isLandmark(el) && resolvesUniquely(el, el.localName)) {
		return el.localName;
	}

	const landmark = nearestLandmark(el);
	if (landmark && landmark !== el) {
		const landmarkSelector = buildSelectorPath(landmark);
		const chain = relativeChain(landmark, el);
		if (chain) {
			const candidate = `${landmarkSelector} > ${chain}`;
			if (resolvesUniquely(el, candidate)) {
				return candidate;
			}
		}
	}

	return absoluteChain(el);
}

function isLandmark(el: Element): boolean {
	for (const selector of LANDMARK_SELECTORS) {
		try {
			if (el.matches(selector)) {
				return true;
			}
		} catch {
			// ignore
		}
	}
	return false;
}

function nearestLandmark(el: Element): Element | null {
	for (const selector of LANDMARK_SELECTORS) {
		const match = el.closest(selector);
		if (match && match !== el) {
			return match;
		}
	}
	return null;
}

function relativeChain(ancestor: Element, target: Element): string | null {
	const steps: string[] = [];
	let cursor: Element | null = target;
	while (cursor && cursor !== ancestor) {
		const parent: Element | null = cursor.parentElement;
		if (!parent) {
			return null;
		}
		steps.unshift(`${cursor.localName}:nth-of-type(${nthOfType(cursor)})`);
		cursor = parent;
	}
	return cursor === ancestor ? steps.join(' > ') : null;
}

function absoluteChain(el: Element): string {
	const steps: string[] = [];
	let cursor: Element | null = el;
	while (cursor && cursor.localName !== 'html') {
		steps.unshift(`${cursor.localName}:nth-of-type(${nthOfType(cursor)})`);
		cursor = cursor.parentElement;
	}
	return steps.length === 0 ? 'html' : `html > ${steps.join(' > ')}`;
}

function nthOfType(el: Element): number {
	const parent = el.parentElement;
	if (!parent) {
		return 1;
	}
	let n = 0;
	for (const child of Array.from(parent.children)) {
		if (child.localName === el.localName) {
			n += 1;
			if (child === el) {
				return n;
			}
		}
	}
	return n;
}

function resolvesUniquely(target: Element, selector: string): boolean {
	try {
		const root = target.ownerDocument ?? document;
		const matches = root.querySelectorAll(selector);
		return matches.length === 1 && matches[0] === target;
	} catch {
		// Invalid selector for some odd id / attribute value — fall through
		// to the next strategy.
		return false;
	}
}

function cssEscapeAttr(value: string): string {
	return value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}
