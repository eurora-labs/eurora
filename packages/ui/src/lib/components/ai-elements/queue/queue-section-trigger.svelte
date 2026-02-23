<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CollapsibleTrigger } from '$lib/components/collapsible/index.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLButtonAttributes & {
		children?: Snippet;
	} = $props();
</script>

<CollapsibleTrigger>
	<button
		data-slot="queue-section-trigger"
		class={cn(
			'group flex w-full items-center justify-between rounded-md bg-muted/40 px-3 py-2 text-left font-medium text-muted-foreground text-sm transition-colors hover:bg-muted',
			className,
		)}
		type="button"
		{...restProps}
	>
		{@render children?.()}
	</button>
</CollapsibleTrigger>
