import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import { roles as ariaRoles } from 'aria-query';
import { isVisible } from '../../extensions/web/element-filter';
import { buildSelectorPath } from '../../extensions/web/selector-path';
import {
	AX_TREE_DEFAULT_DEPTH,
	AX_TREE_DEFAULT_NODES,
	AX_TREE_HARD_CAP_NODES,
	clampNodeBudget,
} from '../../extensions/web/truncation';
import type { Tool } from '../types';

const Args = z
	.object({
		root_selector: z.string().min(1).optional(),
		max_depth: z.number().int().positive().optional(),
		max_nodes: z.number().int().positive().optional(),
	})
	.strict();

interface AxNodeT {
	role: string;
	name: string | null;
	value: string | null;
	description: string | null;
	selector_path: string | null;
	children: AxNodeT[];
}

/// Recursive schema. Zod's variance-strict typing can't bidirectionally
/// match the inferred `{ children: z.ZodArray<this> }` against the
/// explicit recursive interface; the cast asserts the equivalence we
/// know holds structurally.
const AxNodeSchema: z.ZodType<AxNodeT, z.ZodTypeDef, AxNodeT> = z.lazy(() =>
	z.object({
		role: z.string(),
		name: z.string().nullable(),
		value: z.string().nullable(),
		description: z.string().nullable(),
		selector_path: z.string().nullable(),
		children: z.array(AxNodeSchema),
	}),
) as z.ZodType<AxNodeT, z.ZodTypeDef, AxNodeT>;

const Out = z.object({
	root: AxNodeSchema,
	node_count: z.number().int().nonnegative(),
	truncated: z.boolean(),
});

type Result = z.infer<typeof Out>;

function resolveRoot(selector: string | undefined): Element | null {
	if (!selector) return document.body;
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

function isPageBoundary(el: Element): boolean {
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

function computeRole(el: Element): string {
	const explicit = el.getAttribute('role');
	if (explicit) {
		const candidate = explicit.split(/\s+/).find((token) => ariaRoles.has(token));
		if (candidate) return candidate;
	}
	const implicit = IMPLICIT_ROLES[el.localName];
	if (typeof implicit === 'string') return implicit;
	if (typeof implicit === 'function') return implicit(el) ?? 'generic';
	return 'generic';
}

function nativeLabelText(
	el: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement,
): string | null {
	if (el.labels && el.labels.length > 0) {
		const parts: string[] = [];
		for (const label of Array.from(el.labels)) {
			const text = (label.textContent ?? '').trim();
			if (text) parts.push(text);
		}
		if (parts.length > 0) return parts.join(' ');
	}
	const ancestor = el.closest('label');
	if (ancestor) {
		const text = (ancestor.textContent ?? '').trim();
		if (text) return text;
	}
	return null;
}

function resolveIdRefs(doc: Document, refs: string, seen: Set<Element>): string | null {
	const parts: string[] = [];
	for (const id of refs.split(/\s+/)) {
		if (!id) continue;
		const target = doc.getElementById(id);
		if (!target) continue;
		const name = computeAccessibleName(target, seen);
		if (name) parts.push(name);
	}
	return parts.length > 0 ? parts.join(' ') : null;
}

function collectTextContent(el: Element, seen: Set<Element>): string {
	const parts: string[] = [];
	for (const child of Array.from(el.childNodes)) {
		if (child.nodeType === Node.TEXT_NODE) {
			const text = child.textContent?.trim();
			if (text) parts.push(text);
			continue;
		}
		if (child.nodeType === Node.ELEMENT_NODE) {
			const childEl = child as Element;
			if (!isVisible(childEl)) continue;
			const childName = computeAccessibleName(childEl, seen);
			if (childName) parts.push(childName);
		}
	}
	return parts.join(' ').replace(/\s+/g, ' ').trim();
}

function computeAccessibleName(el: Element, seen: Set<Element> = new Set()): string | null {
	if (seen.has(el)) return null;
	seen.add(el);

	const labelledBy = el.getAttribute('aria-labelledby');
	if (labelledBy) {
		const text = resolveIdRefs(el.ownerDocument ?? document, labelledBy, seen);
		if (text) return text;
	}

	const ariaLabel = el.getAttribute('aria-label')?.trim();
	if (ariaLabel) return ariaLabel;

	if (
		el instanceof HTMLInputElement ||
		el instanceof HTMLTextAreaElement ||
		el instanceof HTMLSelectElement
	) {
		const labelText = nativeLabelText(el);
		if (labelText) return labelText;
		const placeholder = (el as HTMLInputElement | HTMLTextAreaElement).placeholder?.trim();
		if (placeholder) return placeholder;
	}

	if (el instanceof HTMLImageElement && el.alt) return el.alt.trim();

	const title = el.getAttribute('title')?.trim();
	if (title) return title;

	const textContent = collectTextContent(el, seen);
	return textContent.length > 0 ? textContent : null;
}

function computeValue(el: Element): string | null {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) return el.value;
	if (el instanceof HTMLSelectElement) return el.value;
	return el.getAttribute('aria-valuetext') ?? el.getAttribute('aria-valuenow');
}

function computeDescription(el: Element): string | null {
	const describedBy = el.getAttribute('aria-describedby');
	if (describedBy) {
		const text = resolveIdRefs(el.ownerDocument ?? document, describedBy, new Set());
		if (text) return text;
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

function buildAxNode(
	el: Element,
	depth: number,
	maxDepth: number,
	maxNodes: number,
	counter: Counter,
): AxNodeT {
	counter.count += 1;

	const role = computeRole(el);
	const name = computeAccessibleName(el);
	const value = computeValue(el);
	const description = computeDescription(el);
	const selectorPath = safeSelectorPath(el);

	const children: AxNodeT[] = [];
	if (depth >= maxDepth) {
		counter.truncated = true;
		return { role, name, value, description, selector_path: selectorPath, children };
	}

	for (const child of Array.from(el.children)) {
		if (counter.count >= maxNodes) {
			counter.truncated = true;
			break;
		}
		if (!isVisible(child)) continue;
		children.push(buildAxNode(child, depth + 1, maxDepth, maxNodes, counter));
	}

	return { role, name, value, description, selector_path: selectorPath, children };
}

export async function executeGetAccessibilityTree(args: z.infer<typeof Args>): Promise<Result> {
	const root = resolveRoot(args.root_selector);
	if (!root) {
		throw new Error(`root_selector "${args.root_selector ?? '<body>'}" matched no element`);
	}

	const maxDepth = clampNodeBudget(args.max_depth, AX_TREE_DEFAULT_DEPTH, AX_TREE_DEFAULT_DEPTH);
	const maxNodes = clampNodeBudget(args.max_nodes, AX_TREE_DEFAULT_NODES, AX_TREE_HARD_CAP_NODES);

	const counter = { count: 0, truncated: false };
	const tree = buildAxNode(root, 0, maxDepth, maxNodes, counter);

	return {
		root: tree,
		node_count: counter.count,
		truncated: counter.truncated,
	};
}

export const getAccessibilityTree: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'web_get_accessibility_tree',
		description:
			"Compact accessibility tree rooted at `root_selector` (defaults to <body>), derived from ARIA attributes and HTML implicit roles. Each node carries role, accessible name, value, description, and a stable selector path. Bounded by `max_depth` (default 12) and `max_nodes` (default 500, hard cap 2000); `truncated` is set when either cap is hit.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 8_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return executeGetAccessibilityTree(args);
	},
};
