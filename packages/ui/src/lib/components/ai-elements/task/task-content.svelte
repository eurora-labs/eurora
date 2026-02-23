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
	data-slot="task-content"
	class={cn(
		'data-[state=closed]:fade-out-0 data-[state=closed]:slide-out-to-top-2 data-[state=open]:slide-in-from-top-2 text-popover-foreground outline-none data-[state=closed]:animate-out data-[state=open]:animate-in',
		className,
	)}
	{...restProps}
>
	<div class="mt-4 space-y-2 border-muted border-l-2 pl-4">
		{@render children?.()}
	</div>
</CollapsibleContent>
