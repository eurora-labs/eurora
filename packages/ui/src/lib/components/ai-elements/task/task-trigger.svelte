<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { CollapsibleTrigger } from '$lib/components/collapsible/index.js';
	import Search from '@lucide/svelte/icons/search';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { Collapsible as CollapsiblePrimitive } from 'bits-ui';

	let {
		class: className,
		title,
		children,
		...restProps
	}: CollapsiblePrimitive.TriggerProps & {
		title: string;
		children?: Snippet;
	} = $props();
</script>

<CollapsibleTrigger
	data-slot="task-trigger"
	class={cn('group', className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<div class="flex w-full cursor-pointer items-center gap-2 text-muted-foreground text-sm transition-colors hover:text-foreground">
			<Search class="size-4" />
			<p class="text-sm">{title}</p>
			<ChevronDown class="size-4 transition-transform group-data-[state=open]:rotate-180" />
		</div>
	{/if}
</CollapsibleTrigger>
