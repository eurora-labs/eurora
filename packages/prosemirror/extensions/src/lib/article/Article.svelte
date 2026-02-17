<script lang="ts" module>
	import { Node as PMNode } from 'prosemirror-model';
	import type { NodeSpec } from 'prosemirror-model';

	export interface Frame {
		id: string;
		ocrText?: string;
	}

	export interface ArticleAttrs {
		id?: string;
		transcript?: string;
		text?: string;
		name?: string;
		currentFrame?: Frame;
	}

	export const articleAttrs: ArticleAttrs = {
		id: undefined,
		transcript: undefined,
		text: undefined,
		name: 'article',
		currentFrame: undefined,
	};

	export const articleSchema: NodeSpec = {
		attrs: Object.entries(articleAttrs).reduce(
			(acc, [key, value]) => ({ ...acc, [key]: { default: value } }),
			{},
		),
		group: 'inline',
		inline: true,
		atom: true,
		selectable: true,

		parseDOM: [
			{
				tag: 'span.article[data-id][data-text][data-name]', // Changed from figure
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return {
							id: dom.getAttribute('data-id'),
							text: dom.getAttribute('data-text'),
							name: dom.getAttribute('data-name'),
						};
					}
					return null;
				},
			},
		],
		toDOM(node: PMNode) {
			const { id, text, name } = node.attrs;
			return [
				'span',
				{
					'data-id': id,
					class: 'article',
					'data-text': text,
					'data-name': 'contextChip',
				},
				name || 'article',
			];
		},
	};
</script>

<script lang="ts">
	import { ContextChip } from '@eurora/ui/custom-components/context-chip/index';
	import type { SvelteNodeViewProps } from '@eurora/prosemirror-core/index';
	export interface Props extends SvelteNodeViewProps<ArticleAttrs> {
		ref: HTMLElement;
		attrs: ArticleAttrs;
	}

	let { ref, attrs }: Props = $props();

	export { ref, attrs, articleAttrs, articleSchema };

	function handleKeyDown(event: KeyboardEvent) {
		event.preventDefault();
	}

	export function destroy() {
		ref?.remove();
	}
</script>

<ContextChip bind:ref data-hole {...attrs} onkeydown={handleKeyDown}>{attrs.name}</ContextChip>
<!-- <ContextChip bind:ref data-hole {...attrs} onkeydown={handleKeyDown}
	>www.hydration-water.com</ContextChip -->
>
