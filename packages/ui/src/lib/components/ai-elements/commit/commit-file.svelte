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
	data-slot="commit-file"
	class={cn(
		'flex items-center justify-between gap-2 rounded px-2 py-1 hover:bg-muted/50',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</div>
