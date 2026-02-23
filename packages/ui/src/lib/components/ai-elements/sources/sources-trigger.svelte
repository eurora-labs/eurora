<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { CollapsibleTrigger } from '$lib/components/collapsible/index.js';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';

	let {
		class: className,
		count,
		children,
		...restProps
	}: { class?: string; count: number; children?: Snippet; [key: string]: any } = $props();
</script>

<CollapsibleTrigger
	data-slot="sources-trigger"
	class={cn('flex items-center gap-2', className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<p class="font-medium">Used {count} sources</p>
		<ChevronDownIcon class="h-4 w-4" />
	{/if}
</CollapsibleTrigger>
