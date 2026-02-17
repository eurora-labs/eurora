import { getAttrsWithOutputSpec } from '$lib/extensions/getAttrsWithOutputSpec.js';
import { htmlToDOMOutputSpec } from '$lib/extensions/htmlToDOMOutputSpec.js';
import { type DOMOutputSpec, Node as PMNode, type NodeSpec } from 'prosemirror-model';
import { mount } from 'svelte';
import type { SveltePMNode } from '$lib/typings/index.js';

export async function createNodeSpec(pm_node: SveltePMNode<any>): Promise<NodeSpec> {
	const { schema, component } = pm_node;
	if (component && schema) {
		const div = document.createElement('div');
		// eslint-disable-next-line @typescript-eslint/await-thenable
		const comp = (await mount(component, {
			target: div,
			props: {
				node: pm_node as any,
				attrs: pm_node.attrs,
				contentDOM: () => undefined,
			},
		})) as any;

		const spec = htmlToDOMOutputSpec(comp.ref);
		schema.toDOM = (_node: PMNode) => {
			return spec as unknown as DOMOutputSpec;
		};
		schema.parseDOM = [
			...(schema.parseDOM || []),
			{
				tag: comp.ref.tagName.toLowerCase(),
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return getAttrsWithOutputSpec(spec, dom, {
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
