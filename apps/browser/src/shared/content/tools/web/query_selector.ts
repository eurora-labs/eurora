import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import { isDenylisted, isVisible } from '../../extensions/web/element-filter';
import { buildSelectorPath } from '../../extensions/web/selector-path';
import { HTML_NODE_CAP, TEXT_NODE_CAP, clampString } from '../../extensions/web/truncation';
import type { Tool } from '../types';

const DEFAULT_LIMIT = 50;
const HARD_LIMIT = 500;
const VALID_INCLUDES = ['text', 'html', 'attributes', 'bounds'] as const;
type IncludeKey = (typeof VALID_INCLUDES)[number];

const Args = z
	.object({
		selector: z.string().min(1),
		limit: z.number().int().positive().optional(),
		include: z.array(z.enum(VALID_INCLUDES)).optional(),
	})
	.strict();

const Bounds = z.object({
	x: z.number().nullable(),
	y: z.number().nullable(),
	width: z.number().nullable(),
	height: z.number().nullable(),
});

const DomNode = z.object({
	selector_path: z.string(),
	text: z.string().nullable(),
	html: z.string().nullable(),
	attributes: z.record(z.string()).nullable(),
	bounds: Bounds.nullable(),
});

const Out = z.object({
	matches: z.array(DomNode),
	total_match_count: z.number().int().nonnegative(),
	truncated: z.boolean(),
});

type Result = z.infer<typeof Out>;
type DomNodeT = z.infer<typeof DomNode>;
type BoundsT = z.infer<typeof Bounds>;

function clampLimit(raw: number): number {
	return Math.min(raw, HARD_LIMIT);
}

function readAttributes(el: Element): Record<string, string> {
	const result: Record<string, string> = {};
	for (const attr of Array.from(el.attributes)) {
		result[attr.name] = attr.value;
	}
	return result;
}

function numericOrNull(value: number): number | null {
	return Number.isFinite(value) ? value : null;
}

function readBounds(el: Element): BoundsT | null {
	if (typeof (el as Element & { getBoundingClientRect?: () => DOMRect }).getBoundingClientRect !==
		'function') {
		return null;
	}
	const rect = el.getBoundingClientRect();
	return {
		x: numericOrNull(rect.x),
		y: numericOrNull(rect.y),
		width: numericOrNull(rect.width),
		height: numericOrNull(rect.height),
	};
}

function safeSelectorPath(el: Element): string {
	try {
		return buildSelectorPath(el);
	} catch {
		return '';
	}
}

function projectNode(el: Element, include: Set<IncludeKey>): { node: DomNodeT; truncated: boolean } {
	const node: DomNodeT = {
		selector_path: safeSelectorPath(el),
		text: null,
		html: null,
		attributes: null,
		bounds: null,
	};
	let truncated = false;
	if (include.has('text')) {
		const clamp = clampString(el.textContent ?? '', TEXT_NODE_CAP);
		node.text = clamp.value;
		truncated ||= clamp.truncated;
	}
	if (include.has('html')) {
		const clamp = clampString(el.outerHTML ?? '', HTML_NODE_CAP);
		node.html = clamp.value;
		truncated ||= clamp.truncated;
	}
	if (include.has('attributes')) {
		node.attributes = readAttributes(el);
	}
	if (include.has('bounds')) {
		node.bounds = readBounds(el);
	}
	return { node, truncated };
}

export async function executeQuerySelector(args: z.infer<typeof Args>): Promise<Result> {
	let raw: NodeListOf<Element>;
	try {
		raw = document.querySelectorAll(args.selector);
	} catch (err) {
		const detail = err instanceof Error ? err.message : String(err);
		throw new Error(`query_selector "${args.selector}" is not a valid CSS selector: ${detail}`);
	}

	const limit = clampLimit(args.limit ?? DEFAULT_LIMIT);
	const include = new Set<IncludeKey>(args.include ?? []);

	const matches: DomNodeT[] = [];
	let truncated = false;

	for (const el of Array.from(raw)) {
		if (matches.length >= limit) {
			truncated = true;
			break;
		}
		if (isDenylisted(el)) continue;
		if (!isVisible(el)) continue;
		const projected = projectNode(el, include);
		if (projected.truncated) truncated = true;
		matches.push(projected.node);
	}

	return {
		matches,
		total_match_count: raw.length,
		truncated,
	};
}

export const querySelector: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'web_query_selector',
		description:
			"Return matched elements for a CSS selector, optionally with their text content, outer HTML, attributes, and bounding box. Hidden and safety-denylisted elements are elided. `truncated` indicates that some matches or per-node bodies were trimmed.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 5_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return executeQuerySelector(args);
	},
};
