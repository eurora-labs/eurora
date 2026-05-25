import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	text: z.string(),
	anchor_xpath: z.string().nullable(),
	focus_xpath: z.string().nullable(),
});

type Result = z.infer<typeof Out>;

function isElementNamed(node: Node, localName: string): boolean {
	return node.nodeType === Node.ELEMENT_NODE && (node as Element).localName === localName;
}

function positionAmongSiblings(node: Node, predicate: (sibling: Node) => boolean): number {
	let index = 0;
	const parent = node.parentNode;
	if (!parent) return 1;
	for (let i = 0; i < parent.childNodes.length; i += 1) {
		const sibling = parent.childNodes[i];
		if (predicate(sibling)) {
			index += 1;
			if (sibling === node) return index;
		}
	}
	return index || 1;
}

function stepFor(node: Node): string | null {
	if (node.nodeType === Node.ELEMENT_NODE) {
		const el = node as Element;
		const index = positionAmongSiblings(el, (sib) => isElementNamed(sib, el.localName));
		return `${el.localName}[${index}]`;
	}
	if (node.nodeType === Node.TEXT_NODE) {
		const index = positionAmongSiblings(node, (sib) => sib.nodeType === Node.TEXT_NODE);
		return `text()[${index}]`;
	}
	return null;
}

function xpathOf(node: Node): string | null {
	if (!node.ownerDocument) return null;
	const steps: string[] = [];
	let cursor: Node | null = node;
	while (cursor && cursor.nodeType !== Node.DOCUMENT_NODE) {
		const step = stepFor(cursor);
		if (!step) return null;
		steps.unshift(step);
		cursor = cursor.parentNode;
	}
	return steps.length > 0 ? `/${steps.join('/')}` : null;
}

/// Resolve whatever the user has highlighted right now, plus XPaths to
/// the selection's anchor and focus nodes so the model can reason about
/// where in the document the highlight sits. Empty selections (the
/// common case) return an empty `text` and `null` XPath fields — the
/// tool never fails for "nothing selected".
export async function executeGetSelectedText(): Promise<Result> {
	const selection = window.getSelection();
	const text = selection?.toString() ?? '';
	const anchor = selection?.anchorNode ? xpathOf(selection.anchorNode) : null;
	const focus = selection?.focusNode ? xpathOf(selection.focusNode) : null;
	return { text, anchor_xpath: anchor, focus_xpath: focus };
}

export const getSelectedText: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'web_get_selected_text',
		description:
			"Return the user's current text selection on the active page, plus XPaths to the selection's anchor and focus nodes. Empty selections return empty text without erroring.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 500,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return executeGetSelectedText();
	},
};
