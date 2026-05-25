import { isVisible } from '../../extensions/web/element-filter';
import { buildSelectorPath } from '../../extensions/web/selector-path';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const DEFAULT_LIMIT = 100;
const HARD_LIMIT = 500;

const Args = z
	.object({
		root_selector: z.string().min(1).optional(),
		limit: z.number().int().positive().optional(),
	})
	.strict();

const Link = z.object({
	url: z.string(),
	label: z.string().nullable(),
	role: z.string(),
	selector_path: z.string(),
});

const Out = z.object({
	links: z.array(Link),
	total: z.number().int().nonnegative(),
});

type Result = z.infer<typeof Out>;
type LinkT = z.infer<typeof Link>;

function resolveRoot(selector: string | undefined): Element | null {
	if (!selector) return document.body;
	try {
		return document.querySelector(selector);
	} catch {
		return null;
	}
}

function resolveHref(anchor: HTMLAnchorElement, href: string): string | null {
	const trimmed = href.trim();
	if (!trimmed) return null;
	const lower = trimmed.toLowerCase();
	if (
		lower.startsWith('javascript:') ||
		lower.startsWith('mailto:') ||
		lower.startsWith('tel:')
	) {
		return null;
	}
	if (lower.startsWith('#')) return null;
	try {
		const url = new URL(anchor.href);
		if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
		return url.toString();
	} catch {
		return null;
	}
}

function labelOf(anchor: HTMLAnchorElement): string | null {
	const ariaLabel = anchor.getAttribute('aria-label')?.trim();
	if (ariaLabel) return ariaLabel;
	const text = (anchor.textContent ?? '').replace(/\s+/g, ' ').trim();
	if (text) return text;
	const title = anchor.getAttribute('title')?.trim();
	if (title) return title;
	const img = anchor.querySelector('img[alt]');
	const alt = img?.getAttribute('alt')?.trim();
	return alt ? alt : null;
}

function safeSelectorPath(el: Element): string {
	try {
		return buildSelectorPath(el);
	} catch {
		return '';
	}
}

export async function executeListLinks(args: z.infer<typeof Args>): Promise<Result> {
	const root = resolveRoot(args.root_selector);
	if (!root) {
		throw new Error(`root_selector "${args.root_selector ?? '<body>'}" matched no element`);
	}

	const limit = Math.min(args.limit ?? DEFAULT_LIMIT, HARD_LIMIT);
	const seen = new Set<string>();
	const links: LinkT[] = [];
	let total = 0;

	for (const anchor of Array.from(root.querySelectorAll<HTMLAnchorElement>('a[href]'))) {
		if (!isVisible(anchor)) continue;
		const href = anchor.getAttribute('href');
		if (!href) continue;
		const resolved = resolveHref(anchor, href);
		if (!resolved) continue;
		total += 1;
		if (links.length >= limit) continue;
		const dedupKey = `${resolved} ${labelOf(anchor)}`;
		if (seen.has(dedupKey)) continue;
		seen.add(dedupKey);
		links.push({
			url: resolved,
			label: labelOf(anchor),
			role: anchor.getAttribute('role') ?? 'link',
			selector_path: safeSelectorPath(anchor),
		});
	}

	return { links, total };
}

export const listLinks: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'web_list_links',
		description:
			'Inventory of clickable navigations rooted at `root_selector` (defaults to the whole document). Only resolvable `http(s)` URLs are emitted; hash-only, `javascript:`, `mailto:`, and `tel:` links are excluded. `total` reports the pre-`limit` count so the model can opt into a higher limit when needed.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 3_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return await executeListLinks(args);
	},
};
