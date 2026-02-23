<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { ScrollArea } from '$lib/components/scroll-area/index.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	} = $props();
</script>

<ScrollArea data-slot="queue-list" class={cn('mt-2 -mb-1', className)} {...restProps}>
	<div class="max-h-40 pr-4">
		<ul>
			{@render children?.()}
		</ul>
	</div>
</ScrollArea>
