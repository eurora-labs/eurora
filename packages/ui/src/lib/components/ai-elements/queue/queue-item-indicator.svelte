<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		completed = false,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		completed?: boolean;
	} = $props();
</script>

<span
	data-slot="queue-item-indicator"
	class={cn(
		'mt-0.5 inline-block size-2.5 rounded-full border',
		completed
			? 'border-muted-foreground/20 bg-muted-foreground/10'
			: 'border-muted-foreground/50',
		className,
	)}
	{...restProps}
></span>
