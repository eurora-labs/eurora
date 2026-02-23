<script lang="ts" module>
	import type { NodeSpec } from 'prosemirror-model';

	export interface ParagraphAttrs {
		indent: number;
	}

	export const paragraphAttrs: ParagraphAttrs = {
		indent: 0,
	};

	export const paragraphSchema: NodeSpec = {
		attrs: {
			indent: { default: 0 },
		},
		content: 'inline*',
		group: 'block',
	};
</script>

<script lang="ts">
	export interface Props {
		attrs: ParagraphAttrs;
		ref: HTMLParagraphElement;
	}

	let { attrs, ref }: Props = $props();

	export { attrs, ref };
</script>

<p data-indent={attrs.indent} data-hole bind:this={ref}></p>
