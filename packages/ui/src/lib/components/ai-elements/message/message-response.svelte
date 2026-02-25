<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Streamdown, type StreamdownProps } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import Math from 'svelte-streamdown/math';
	import Mermaid from 'svelte-streamdown/mermaid';

	let {
		class: className,
		components,
		...restProps
	}: StreamdownProps & {
		class?: string;
	} = $props();

	const defaultComponents = { code: Code, math: Math, mermaid: Mermaid };
	const mergedComponents = $derived(
		components ? { ...defaultComponents, ...components } : defaultComponents,
	);
</script>

<div data-slot="message-response">
	<Streamdown
		class={cn('size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0', className)}
		components={mergedComponents}
		baseTheme="shadcn"
		{...restProps}
	/>
</div>
