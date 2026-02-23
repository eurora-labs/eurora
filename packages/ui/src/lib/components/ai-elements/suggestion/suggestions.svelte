<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { ScrollArea, ScrollAreaScrollbar } from '$lib/components/scroll-area/index.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & { children?: Snippet } = $props();
</script>

<ScrollArea class="w-full overflow-x-auto whitespace-nowrap" {...restProps}>
	<div data-slot="suggestions" class={cn('flex w-max flex-nowrap items-center gap-2', className)}>
		{@render children?.()}
	</div>
	<ScrollAreaScrollbar class="hidden" orientation="horizontal" />
</ScrollArea>
