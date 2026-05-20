import { isDenylisted, isVisible } from './element-filter';
import { buildSelectorPath } from './selector-path';
import { HTML_NODE_CAP, TEXT_NODE_CAP, clampString } from './truncation';
import type {
	BoundingBox,
	DomNode,
	QuerySelectorArgs,
	QuerySelectorInclude,
	QuerySelectorResult,
} from '../../bindings';
import type { NativeResponse } from '../../models';
import type { BrowserObj } from '../watchers/watcher';

const DEFAULT_LIMIT = 50;
const VALID_INCLUDES = new Set<QuerySelectorInclude>(['text', 'html', 'attributes', 'bounds']);

/**
 * Generalised DOM reader. Hidden elements and the static denylist
 * (password / file / hidden / submit-style inputs, CSRF-shaped metas)
 * are elided from results regardless of selector.
 *
 * `total_match_count` reports the pre-filter count so the LLM knows
 * when something was elided. `truncated` is set when the result was
 * trimmed for reasons the caller can fix (limit too low, or per-node
 * text/HTML truncation kicked in) — denylist elision does **not** set
 * `truncated`, because the elements will never appear no matter how
 * the caller widens the call.
 */
export async function handleQuerySelector(obj: BrowserObj): Promise<NativeResponse> {
	const args = parseArgs(obj);
	if (!args.selector || args.selector.trim().length === 0) {
		throw new Error('query_selector requires a non-empty selector');
	}

	let raw: NodeListOf<Element>;
	try {
		raw = document.querySelectorAll(args.selector);
	} catch (err) {
		throw new Error(
			`query_selector "${args.selector}" is not a valid CSS selector: ${describe(err)}`,
		);
	}

	const limit = normaliseLimit(args.limit ?? DEFAULT_LIMIT);
	const include = normaliseInclude(args.include ?? []);

	const matches: DomNode[] = [];
	let truncated = false;

	for (const el of Array.from(raw)) {
		if (matches.length >= limit) {
			truncated = true;
			break;
		}
		if (isDenylisted(el)) {
			continue;
		}
		if (!isVisible(el)) {
			continue;
		}
		const node = projectNode(el, include);
		if (node.truncated) {
			truncated = true;
		}
		matches.push(node.node);
	}

	const result: QuerySelectorResult = {
		matches,
		total_match_count: raw.length,
		truncated,
	};
	return { kind: 'QuerySelectorResult', data: result };
}

function parseArgs(obj: BrowserObj): QuerySelectorArgs {
	const selector = obj['selector'];
	const rawLimit = obj['limit'];
	const rawInclude = obj['include'];
	return {
		selector: typeof selector === 'string' ? selector : '',
		limit: typeof rawLimit === 'number' && Number.isFinite(rawLimit) ? rawLimit : undefined,
		include: Array.isArray(rawInclude)
			? (rawInclude.filter(
					(v): v is QuerySelectorInclude =>
						typeof v === 'string' && VALID_INCLUDES.has(v as QuerySelectorInclude),
				) as QuerySelectorInclude[])
			: undefined,
	};
}

function normaliseLimit(raw: number): number {
	if (!Number.isInteger(raw) || raw <= 0) {
		return DEFAULT_LIMIT;
	}
	// Hard cap matches the AX-tree hard cap so a single tool call can
	// never blow past ~64 KB of bridge payload.
	return Math.min(raw, 500);
}

function normaliseInclude(raw: QuerySelectorInclude[]): Set<QuerySelectorInclude> {
	return new Set(raw.filter((v) => VALID_INCLUDES.has(v)));
}

interface ProjectedNode {
	node: DomNode;
	truncated: boolean;
}

function projectNode(el: Element, include: Set<QuerySelectorInclude>): ProjectedNode {
	const node: DomNode = {
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

function readAttributes(el: Element): Record<string, string> {
	const result: Record<string, string> = {};
	for (const attr of Array.from(el.attributes)) {
		result[attr.name] = attr.value;
	}
	return result;
}

function readBounds(el: Element): BoundingBox | null {
	if (
		typeof (el as Element & { getBoundingClientRect?: () => DOMRect }).getBoundingClientRect !==
		'function'
	) {
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

function numericOrNull(value: number): number | null {
	return Number.isFinite(value) ? value : null;
}

function safeSelectorPath(el: Element): string {
	try {
		return buildSelectorPath(el);
	} catch {
		return '';
	}
}

function describe(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}
