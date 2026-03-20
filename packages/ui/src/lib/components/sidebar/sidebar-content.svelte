<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn, type WithElementRef } from '$lib/utils.js';
	import * as ScrollArea from '$lib/components/scroll-area/index.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLElement>> = $props();
</script>

<ScrollArea.Root
	class={cn('min-h-0 flex-1 group-data-[collapsible=icon]:overflow-hidden', className)}
	scrollbarYClasses="border-s-0"
>
	<div
		bind:this={ref}
		data-slot="sidebar-content"
		data-sidebar="content"
		class="flex flex-col gap-2"
		{...restProps}
	>
		{@render children?.()}
	</div>
</ScrollArea.Root>
