import { DOMOutputSpec, Node as PMNode, NodeSpec } from 'prosemirror-model';
import { SveltePMNode } from '../typings/index.js';
import { mount } from 'svelte';
import { htmlToDOMOutputSpec } from './htmlToDOMOutputSpec.js';
import { getAttrsWithOutputSpec } from './getAttrsWithOutputSpec.js';

export async function createNodeSpec(pm_node: SveltePMNode<any>): Promise<NodeSpec> {
	const { schema, component } = pm_node;
	if (component && schema) {
		const staticSpec = await createSpec(pm_node);
		schema.toDOM = (node: PMNode) => {
			const div = document.createElement('div');
			const comp = mount(component, {
				target: div,
				props: {
					node,
					attrs: node.attrs,
					contentDOM: () => undefined,
				},
			}) as any;
			if (!comp.ref) return staticSpec;
			const spec = htmlToDOMOutputSpec(comp.ref);
			return spec as unknown as DOMOutputSpec;
		};
		schema.parseDOM = [
			...(schema.parseDOM || []),
			{
				tag: staticSpec[0],
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return getAttrsWithOutputSpec(staticSpec, dom, {
							selector: [],
						});
					}
					return null;
				},
			},
		];
	} else if (!component && schema?.toDOM === undefined) {
		throw Error(
			`You must provide either Svelte component or schema.toDOM method for your Svelte PMNode!`,
		);
	}
	return schema;
}

export async function createSpec(node: SveltePMNode<any>): Promise<readonly [string, ...any[]]> {
	const { attrs, component } = node;
	if (!component) {
		return [''];
	}
	const div = document.createElement('div');
	// eslint-disable-next-line @typescript-eslint/await-thenable
	const comp = (await mount(component, {
		target: div,
		props: {
			node: undefined,
			attrs,
			contentDOM: () => undefined,
		} as any,
	})) as any;
	const spec = htmlToDOMOutputSpec(comp.ref);
	return spec as [string, ...any[]];
}
