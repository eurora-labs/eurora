<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		completed = false,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		completed?: boolean;
		children?: Snippet;
	} = $props();
</script>

<span
	data-slot="queue-item-content"
	class={cn(
		'line-clamp-1 grow break-words',
		completed
			? 'text-muted-foreground/50 line-through'
			: 'text-muted-foreground',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</span>
