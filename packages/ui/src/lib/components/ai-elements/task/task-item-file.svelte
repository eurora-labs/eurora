<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();
</script>

<div
	data-slot="task-item-file"
	class={cn(
		'inline-flex items-center gap-1 rounded-md border bg-secondary px-1.5 py-0.5 text-foreground text-xs',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</div>
