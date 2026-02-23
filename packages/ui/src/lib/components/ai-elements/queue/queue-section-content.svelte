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
	data-slot="queue-section-content"
	class={cn(className)}
	{...restProps}
>
	{@render children?.()}
</CollapsibleContent>
