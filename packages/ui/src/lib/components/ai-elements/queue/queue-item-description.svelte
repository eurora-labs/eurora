<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		completed = false,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		completed?: boolean;
		children?: Snippet;
	} = $props();
</script>

<div
	data-slot="queue-item-description"
	class={cn(
		'ml-6 text-xs',
		completed ? 'text-muted-foreground/40 line-through' : 'text-muted-foreground',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</div>
