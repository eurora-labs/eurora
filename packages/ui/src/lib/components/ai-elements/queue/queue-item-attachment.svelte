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
	data-slot="queue-item-attachment"
	class={cn('mt-1 flex flex-wrap gap-2', className)}
	{...restProps}
>
	{@render children?.()}
</div>
