<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { CollapsibleContent } from '$lib/components/collapsible/index.js';
	import { Collapsible as CollapsiblePrimitive } from 'bits-ui';

	let {
		class: className,
		children,
		...restProps
	}: CollapsiblePrimitive.ContentProps & {
		children?: Snippet;
	} = $props();
</script>

<CollapsibleContent
	data-slot="tool-content"
	class={cn(
		'data-[state=closed]:fade-out-0 data-[state=closed]:slide-out-to-top-2 data-[state=open]:slide-in-from-top-2 space-y-4 p-4 text-popover-foreground outline-none data-[state=closed]:animate-out data-[state=open]:animate-in',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</CollapsibleContent>
