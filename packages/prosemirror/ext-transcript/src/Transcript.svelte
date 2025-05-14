<script lang="ts" module>
	import type { NodeSpec } from 'prosemirror-model';
	import { Node as PMNode } from 'prosemirror-model';
	import type { NodeProps } from '@eurora/prosemirror-core';
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
		defining: false,
		atom: false,

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
			return ['span', { id, class: 'transcript' }];
		}
	};
</script>

<script lang="ts">
	import { Badge } from '@eurora/ui';
	import type { SvelteNodeViewProps } from '@eurora/prosemirror-core';
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

	export function destroy() {
		ref?.remove();
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
	.transcript {
		@apply w-fit items-center gap-2 text-[40px] leading-[40px] text-white;
		@apply mx-2 p-2;
		color: rgba(0, 0, 0, 0.7);
		border-radius: 16px;
		display: inline-block;
		backdrop-filter: blur(6px);
		-webkit-backdrop-filter: blur(6px);
	}
</style>
