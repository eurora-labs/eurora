import { handleGetAccessibilityTree } from '../accessibility';
import { describe, it, expect, beforeEach } from 'vitest';
import type { AccessibilityTree, AxNode } from '../../../bindings';
import type { BrowserObj } from '../../watchers/watcher';

function args(overrides: Partial<BrowserObj> = {}): BrowserObj {
	return { type: 'GET_ACCESSIBILITY_TREE', ...overrides };
}

async function call(overrides: Partial<BrowserObj> = {}): Promise<AccessibilityTree> {
	const response = await handleGetAccessibilityTree(args(overrides));
	return response.data as AccessibilityTree;
}

function findFirst(node: AxNode, predicate: (n: AxNode) => boolean): AxNode | null {
	if (predicate(node)) return node;
	for (const child of node.children) {
		const hit = findFirst(child, predicate);
		if (hit) return hit;
	}
	return null;
}

function findAll(node: AxNode, predicate: (n: AxNode) => boolean, acc: AxNode[] = []): AxNode[] {
	if (predicate(node)) acc.push(node);
	for (const child of node.children) findAll(child, predicate, acc);
	return acc;
}

describe('handleGetAccessibilityTree', () => {
	beforeEach(() => {
		document.body.innerHTML = '';
	});

	it('emits an AccessibilityTree envelope', async () => {
		document.body.innerHTML = '<main></main>';
		const response = await handleGetAccessibilityTree(args());
		expect(response.kind).toBe('AccessibilityTree');
	});

	it('resolves implicit roles for native elements', async () => {
		document.body.innerHTML = `
			<main>
				<button>Submit</button>
				<a href="/path">Link</a>
				<input type="text" aria-label="user">
				<input type="checkbox">
			</main>
		`;
		const tree = await call();
		expect(findFirst(tree.root, (n) => n.role === 'button')).not.toBeNull();
		expect(findFirst(tree.root, (n) => n.role === 'link')).not.toBeNull();
		expect(findFirst(tree.root, (n) => n.role === 'textbox')).not.toBeNull();
		expect(findFirst(tree.root, (n) => n.role === 'checkbox')).not.toBeNull();
	});

	it('respects explicit ARIA role attributes', async () => {
		document.body.innerHTML = '<div role="button">Click</div>';
		const tree = await call();
		expect(findFirst(tree.root, (n) => n.role === 'button')).not.toBeNull();
	});

	it('computes accessible-name via aria-labelledby', async () => {
		document.body.innerHTML = `
			<span id="lbl">Username</span>
			<input type="text" aria-labelledby="lbl">
		`;
		const tree = await call();
		const node = findFirst(tree.root, (n) => n.role === 'textbox');
		expect(node?.name).toBe('Username');
	});

	it('prunes subtrees hidden via aria-hidden', async () => {
		document.body.innerHTML = `
			<main>
				<button>Visible</button>
				<div aria-hidden="true">
					<button>Hidden</button>
				</div>
			</main>
		`;
		const tree = await call();
		const buttons = findAll(tree.root, (n) => n.role === 'button');
		expect(buttons).toHaveLength(1);
	});

	it('sets truncated=true when max_nodes is exceeded', async () => {
		// 6 buttons; cap at 3 — counter increments on every visit, so the
		// cap kicks in before all are emitted.
		document.body.innerHTML = Array.from({ length: 6 }, () => '<button>b</button>').join('');
		const tree = await call({ max_nodes: 3 });
		expect(tree.truncated).toBe(true);
		expect(tree.node_count).toBeLessThanOrEqual(3);
	});

	it('honours root_selector to scope the traversal', async () => {
		document.body.innerHTML = `
			<aside>
				<button>Aside</button>
			</aside>
			<main id="m">
				<button>Inside</button>
			</main>
		`;
		const tree = await call({ root_selector: '#m' });
		const labels = findAll(tree.root, () => true).map((n) => n.name);
		expect(labels.includes('Inside')).toBe(true);
		expect(labels.includes('Aside')).toBe(false);
	});

	it('rejects unresolvable root_selector with a structured error', async () => {
		await expect(handleGetAccessibilityTree(args({ root_selector: '#nope' }))).rejects.toThrow(
			/matched no element/,
		);
	});
});
