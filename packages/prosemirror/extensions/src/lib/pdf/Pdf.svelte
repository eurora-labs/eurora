<script lang="ts" module>
	import { Node as PMNode } from 'prosemirror-model';
	import type { NodeSpec } from 'prosemirror-model';

	export interface Frame {
		id: string;
		ocrText?: string;
	}

	export interface PdfAttrs {
		id?: string;
		name?: string;
		content?: string;
	}

	export const pdfAttrs: PdfAttrs = {
		id: undefined,
		name: 'pdf',
		content: undefined,
	};

	export const pdfSchema: NodeSpec = {
		attrs: Object.entries(pdfAttrs).reduce(
			(acc, [key, value]) => ({ ...acc, [key]: { default: value } }),
			{},
		),
		// content: 'inline+',
		group: 'inline',
		inline: true,
		atom: true,
		selectable: true,

		parseDOM: [
			{
				tag: 'span.pdf[data-id][data-content][data-name]', // Changed from figure
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return {
							id: dom.getAttribute('data-id'),
							content: dom.getAttribute('data-content'),
							name: dom.getAttribute('data-name'),
						};
					}
					return null;
				},
			},
		],
		toDOM(node: PMNode) {
			const { id, content, name } = node.attrs;
			return [
				'span',
				{
					'data-id': id,
					class: 'pdf',
					'data-content': content,
					'data-name': 'contextChip',
				},
				name || 'pdf', // Add the text content as the third element
			];
		},
	};
</script>

<script lang="ts">
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Popover from '@eurora/ui/components/popover/index';
	import { ContextChip } from '@eurora/ui/custom-components/context-chip/index';
	import type { SvelteNodeViewProps } from '@eurora/prosemirror-core/index';
	export interface Props extends SvelteNodeViewProps<PdfAttrs> {
		ref: HTMLElement;
		attrs: PdfAttrs;
	}

	let { ref, attrs }: Props = $props();

	export { ref, attrs, pdfAttrs, pdfSchema };

	function handleClick(event: MouseEvent) {
		alert('some longer script');
		event.preventDefault();
	}

	function handleKeyDown(event: KeyboardEvent) {
		event.preventDefault();
	}

	export function destroy() {
		ref?.remove();
	}
</script>

<ContextChip bind:ref data-hole {...attrs} onkeydown={handleKeyDown}>{attrs.name}</ContextChip>
