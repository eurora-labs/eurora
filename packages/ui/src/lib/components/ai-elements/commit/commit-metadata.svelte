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
	data-slot="commit-metadata"
	class={cn('flex items-center gap-2 text-muted-foreground text-xs', className)}
	{...restProps}
>
	{@render children?.()}
</div>
