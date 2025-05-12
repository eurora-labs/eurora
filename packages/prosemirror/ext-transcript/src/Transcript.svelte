<script lang="ts" module>
	import type { NodeSpec } from 'prosemirror-model';
	import { Node as PMNode } from 'prosemirror-model';
	import type { NodeProps } from '../../typings/extension.js';
	export interface TranscriptAttrs {
		id?: string;
		text?: string;
	}

	export const transcriptAttrs: TranscriptAttrs = {
		id: undefined,
		text: undefined
	};

	export const transcriptSchema: NodeSpec = {
		attrs: Object.entries(transcriptAttrs).reduce(
			(acc, [key, value]) => ({ ...acc, [key]: { default: value } }),
			{}
		),
		content: 'inline+',
		group: 'inline',
		inline: true,

		parseDOM: [
			{
				tag: 'span.transcript', // Changed from figure
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return {
							id: dom.getAttribute('id'),
							text: dom.getAttribute('data-text')
						};
					}
					return null;
				}
			}
		],
		toDOM(node: PMNode) {
			const { id, text } = node.attrs;
			return ['span', { id, class: 'transcript', pre: text }, text];
		}
	};
</script>

<script lang="ts">
	import type { SvelteNodeViewProps } from '../../SvelteNodeView.js';
	export interface Props extends SvelteNodeViewProps<TranscriptAttrs> {
		ref: HTMLElement;
		attrs: TranscriptAttrs;
	}

	let { ref, attrs }: Props = $props();

	export { ref, attrs };

	function handleClick(event: MouseEvent) {
		alert('some longer script');
		event.preventDefault();
	}

	function handleKeyDown(event: KeyboardEvent) {
		event.preventDefault();
	}
</script>

<span
	role="button"
	class="transcript"
	onclick={handleClick}
	bind:this={ref}
	data-hole
	{...attrs}
	onkeydown={handleKeyDown}
>
	{attrs.text}
</span>

<style lang="postcss">
	:global(.transcript) {
		width: fit-content;
		background-color: red;
		padding: 5px;
		display: inline-block;
	}
</style>
