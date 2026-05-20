import { isVisible } from './element-filter';
import { buildSelectorPath } from './selector-path';
import {
	AX_TREE_DEFAULT_DEPTH,
	AX_TREE_DEFAULT_NODES,
	AX_TREE_HARD_CAP_NODES,
	clampNodeBudget,
} from './truncation';
import { roles as ariaRoles } from 'aria-query';
import type { AccessibilityTree, AxNode, GetAccessibilityTreeArgs } from '../../bindings';
import type { NativeResponse } from '../../models';
import type { BrowserObj } from '../watchers/watcher';

/**
 * Compact accessibility tree, derived from ARIA attributes and a
 * curated implicit-role table.
 *
 * This is not a CDP-quality AX tree: there is no platform call,
 * focus/state tracking, or live MutationObserver. It is the kind of
 * approximation agentic stacks fall back to when the protocol is
 * unavailable — good enough for form / landmark navigation on
 * accessible sites.
 *
 * Traversal is bounded by `max_depth` (default 12) and `max_nodes`
 * (default 500, hard cap 2 000). When either cap is hit the
 * corresponding subtree is elided and the top-level `truncated` flag is
 * raised.
 */
export async function handleGetAccessibilityTree(obj: BrowserObj): Promise<NativeResponse> {
	const args = parseArgs(obj);
	const root = resolveRoot(args.root_selector);
	if (!root) {
		throw new Error(`root_selector "${args.root_selector ?? '<body>'}" matched no element`);
	}

	const maxDepth = clampNodeBudget(args.max_depth, AX_TREE_DEFAULT_DEPTH, AX_TREE_DEFAULT_DEPTH);
	const maxNodes = clampNodeBudget(args.max_nodes, AX_TREE_DEFAULT_NODES, AX_TREE_HARD_CAP_NODES);

	const counter = { count: 0, truncated: false };
	const tree = buildAxNode(root, 0, maxDepth, maxNodes, counter);

	const result: AccessibilityTree = {
		root: tree,
		node_count: counter.count,
		truncated: counter.truncated,
	};
	return { kind: 'AccessibilityTree', data: result };
}

function parseArgs(obj: BrowserObj): GetAccessibilityTreeArgs {
	return {
		root_selector: stringOrNull(obj['root_selector']),
		max_depth: numberOrNull(obj['max_depth']),
		max_nodes: numberOrNull(obj['max_nodes']),
	};
}

function stringOrNull(value: unknown): string | null {
	return typeof value === 'string' && value.length > 0 ? value : null;
}

function numberOrNull(value: unknown): number | null {
	return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function resolveRoot(selector: string | null): Element | null {
	if (!selector) {
		return document.body;
	}
	try {
		return document.querySelector(selector);
	} catch {
		return null;
	}
}

interface Counter {
	count: number;
	truncated: boolean;
}

function buildAxNode(
	el: Element,
	depth: number,
	maxDepth: number,
	maxNodes: number,
	counter: Counter,
): AxNode {
	counter.count += 1;

	const role = computeRole(el);
	const name = computeAccessibleName(el);
	const value = computeValue(el);
	const description = computeDescription(el);
	const selectorPath = safeSelectorPath(el);

	const children: AxNode[] = [];
	if (depth >= maxDepth) {
		counter.truncated = true;
		return { role, name, value, description, selector_path: selectorPath, children };
	}

	for (const child of Array.from(el.children)) {
		if (counter.count >= maxNodes) {
			counter.truncated = true;
			break;
		}
		if (!isVisible(child)) {
			continue;
		}
		children.push(buildAxNode(child, depth + 1, maxDepth, maxNodes, counter));
	}

	return { role, name, value, description, selector_path: selectorPath, children };
}

/**
 * Implicit role mapping for native HTML elements. Sourced from the
 * [HTML AAM](https://www.w3.org/TR/html-aam-1.0/) and cross-checked
 * against `aria-query`'s `elementRoles`. We keep a hand-rolled table
 * rather than iterating `elementRoles` on every node so the AX-tree hot
 * path stays O(depth * children) without an allocation per lookup.
 */
const IMPLICIT_ROLES: Record<string, string | ((el: Element) => string | null)> = {
	a: (el) => (el.hasAttribute('href') ? 'link' : 'generic'),
	article: 'article',
	aside: 'complementary',
	button: 'button',
	dd: 'definition',
	details: 'group',
	dialog: 'dialog',
	dl: 'list',
	dt: 'term',
	fieldset: 'group',
	figure: 'figure',
	footer: (el) => (isPageBoundary(el) ? 'contentinfo' : 'generic'),
	form: 'form',
	h1: 'heading',
	h2: 'heading',
	h3: 'heading',
	h4: 'heading',
	h5: 'heading',
	h6: 'heading',
	header: (el) => (isPageBoundary(el) ? 'banner' : 'generic'),
	hr: 'separator',
	img: (el) => {
		const alt = el.getAttribute('alt');
		return alt === '' ? 'presentation' : 'img';
	},
	input: inputRole,
	li: 'listitem',
	main: 'main',
	nav: 'navigation',
	ol: 'list',
	option: 'option',
	output: 'status',
	p: 'paragraph',
	progress: 'progressbar',
	search: 'search',
	section: (el) =>
		el.hasAttribute('aria-label') || el.hasAttribute('aria-labelledby') ? 'region' : 'generic',
	select: (el) =>
		(el as HTMLSelectElement).multiple || ((el as HTMLSelectElement).size ?? 0) > 1
			? 'listbox'
			: 'combobox',
	summary: 'button',
	table: 'table',
	tbody: 'rowgroup',
	td: 'cell',
	textarea: 'textbox',
	tfoot: 'rowgroup',
	th: 'columnheader',
	thead: 'rowgroup',
	tr: 'row',
	ul: 'list',
};

function inputRole(el: Element): string {
	const type = (el.getAttribute('type') ?? 'text').toLowerCase();
	switch (type) {
		case 'button':
		case 'submit':
		case 'reset':
		case 'image':
			return 'button';
		case 'checkbox':
			return 'checkbox';
		case 'radio':
			return 'radio';
		case 'range':
			return 'slider';
		case 'number':
			return 'spinbutton';
		case 'search':
			return 'searchbox';
		case 'email':
		case 'tel':
		case 'url':
		case 'text':
			return 'textbox';
		case 'hidden':
			return 'none';
		default:
			return 'textbox';
	}
}

function isPageBoundary(el: Element): boolean {
	// A <header>/<footer> only contributes a landmark role when it sits
	// at the page level — inside <article>/<section> it's generic.
	let parent: Element | null = el.parentElement;
	while (parent) {
		const tag = parent.localName;
		if (
			tag === 'article' ||
			tag === 'section' ||
			tag === 'main' ||
			tag === 'aside' ||
			tag === 'nav'
		) {
			return false;
		}
		parent = parent.parentElement;
	}
	return true;
}

function computeRole(el: Element): string {
	const explicit = el.getAttribute('role');
	if (explicit) {
		const candidate = explicit.split(/\s+/).find((token) => ariaRoles.has(token));
		if (candidate) {
			return candidate;
		}
	}
	const implicit = IMPLICIT_ROLES[el.localName];
	if (typeof implicit === 'string') {
		return implicit;
	}
	if (typeof implicit === 'function') {
		return implicit(el) ?? 'generic';
	}
	return 'generic';
}

/**
 * Accessible-name resolution per the [accname spec](https://w3c.github.io/accname/),
 * pared down to the steps that bear on what the model can see:
 *
 *   1. `aria-labelledby` — fetch and join the referenced elements'
 *      accessible names.
 *   2. `aria-label`.
 *   3. Native control labels: `<label for>`, ancestor `<label>`.
 *   4. `placeholder` (HTML & ARIA).
 *   5. `title`.
 *   6. Text content (depth-bounded; recursion guarded against cycles).
 */
function computeAccessibleName(el: Element, seen: Set<Element> = new Set()): string | null {
	if (seen.has(el)) {
		return null;
	}
	seen.add(el);

	const labelledBy = el.getAttribute('aria-labelledby');
	if (labelledBy) {
		const text = resolveIdRefs(el.ownerDocument ?? document, labelledBy, seen);
		if (text) {
			return text;
		}
	}

	const ariaLabel = el.getAttribute('aria-label')?.trim();
	if (ariaLabel) {
		return ariaLabel;
	}

	if (
		el instanceof HTMLInputElement ||
		el instanceof HTMLTextAreaElement ||
		el instanceof HTMLSelectElement
	) {
		const labelText = nativeLabelText(el);
		if (labelText) {
			return labelText;
		}
		const placeholder = (el as HTMLInputElement | HTMLTextAreaElement).placeholder?.trim();
		if (placeholder) {
			return placeholder;
		}
	}

	if (el instanceof HTMLImageElement && el.alt) {
		return el.alt.trim();
	}

	const title = el.getAttribute('title')?.trim();
	if (title) {
		return title;
	}

	const textContent = collectTextContent(el, seen);
	return textContent.length > 0 ? textContent : null;
}

function nativeLabelText(
	el: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement,
): string | null {
	if (el.labels && el.labels.length > 0) {
		const parts: string[] = [];
		for (const label of Array.from(el.labels)) {
			const text = (label.textContent ?? '').trim();
			if (text) {
				parts.push(text);
			}
		}
		if (parts.length > 0) {
			return parts.join(' ');
		}
	}
	const ancestor = el.closest('label');
	if (ancestor) {
		const text = (ancestor.textContent ?? '').trim();
		if (text) {
			return text;
		}
	}
	return null;
}

function resolveIdRefs(doc: Document, refs: string, seen: Set<Element>): string | null {
	const parts: string[] = [];
	for (const id of refs.split(/\s+/)) {
		if (!id) {
			continue;
		}
		const target = doc.getElementById(id);
		if (!target) {
			continue;
		}
		const name = computeAccessibleName(target, seen);
		if (name) {
			parts.push(name);
		}
	}
	return parts.length > 0 ? parts.join(' ') : null;
}

function collectTextContent(el: Element, seen: Set<Element>): string {
	// Walk children rather than relying on `textContent` so we can apply
	// the accname recursion to nested controls (each contributes its own
	// accessible name, not its raw text).
	const parts: string[] = [];
	for (const child of Array.from(el.childNodes)) {
		if (child.nodeType === Node.TEXT_NODE) {
			const text = child.textContent?.trim();
			if (text) {
				parts.push(text);
			}
			continue;
		}
		if (child.nodeType === Node.ELEMENT_NODE) {
			const childEl = child as Element;
			if (!isVisible(childEl)) {
				continue;
			}
			const childName = computeAccessibleName(childEl, seen);
			if (childName) {
				parts.push(childName);
			}
		}
	}
	return parts.join(' ').replace(/\s+/g, ' ').trim();
}

function computeValue(el: Element): string | null {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		return el.value;
	}
	if (el instanceof HTMLSelectElement) {
		return el.value;
	}
	const aria = el.getAttribute('aria-valuetext') ?? el.getAttribute('aria-valuenow');
	return aria;
}

function computeDescription(el: Element): string | null {
	const describedBy = el.getAttribute('aria-describedby');
	if (describedBy) {
		const text = resolveIdRefs(el.ownerDocument ?? document, describedBy, new Set());
		if (text) {
			return text;
		}
	}
	const title = el.getAttribute('title');
	return title ? title.trim() : null;
}

function safeSelectorPath(el: Element): string | null {
	try {
		return buildSelectorPath(el);
	} catch {
		return null;
	}
}
