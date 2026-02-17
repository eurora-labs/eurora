<script lang="ts" module>
	import { Node as PMNode } from 'prosemirror-model';
	import type { NodeSpec } from 'prosemirror-model';

	export interface Frame {
		id: string;
		ocrText?: string;
	}

	export interface VideoAttrs {
		id?: string;
		transcript?: string;
		text?: string;
		name?: string;
		currentFrame?: Frame;
	}

	export const videoAttrs: VideoAttrs = {
		id: undefined,
		transcript: undefined,
		text: undefined,
		name: undefined,
		currentFrame: undefined,
	};

	export const videoSchema: NodeSpec = {
		attrs: Object.entries(videoAttrs).reduce(
			(acc, [key, value]) => ({ ...acc, [key]: { default: value } }),
			{},
		),
		group: 'inline',
		inline: true,
		atom: true,
		selectable: true,

		parseDOM: [
			{
				tag: 'span.video', // Changed from figure
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return {
							id: dom.getAttribute('id'),
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
			return ['span', { id, 'data-text': text, 'data-name': name, class: 'video' }];
		},
	};
</script>

<script lang="ts">
	import { ContextChip } from '@eurora/ui/custom-components/context-chip/index';
	import { SiYoutube } from '@icons-pack/svelte-simple-icons';
	import type { SvelteNodeViewProps } from '@eurora/prosemirror-core/index';
	export interface Props extends SvelteNodeViewProps<VideoAttrs> {
		ref: HTMLElement;
		attrs: VideoAttrs;
	}

	let { ref, attrs }: Props = $props();

	export { ref, attrs };

	function handleKeyDown(event: KeyboardEvent) {
		event.preventDefault();
	}

	export function destroy() {
		ref?.remove();
	}
</script>

<ContextChip bind:ref data-hole {...attrs} onkeydown={handleKeyDown}>
	<SiYoutube size={24} />
	{attrs.name}
</ContextChip>
