<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';

	let {
		class: className,
		name,
		type,
		required = false,
		description,
		location,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		name: string;
		type: string;
		required?: boolean;
		description?: string;
		location?: 'path' | 'query' | 'header';
	} = $props();
</script>

<div
	data-slot="schema-display-parameter"
	class={cn('px-4 py-3 pl-10', className)}
	{...restProps}
>
	<div class="flex items-center gap-2">
		<span class="font-mono text-sm">{name}</span>
		<Badge class="text-xs" variant="outline">{type}</Badge>
		{#if location}
			<Badge class="text-xs" variant="secondary">{location}</Badge>
		{/if}
		{#if required}
			<Badge
				class="bg-red-100 text-red-700 text-xs dark:bg-red-900/30 dark:text-red-400"
				variant="secondary"
			>
				required
			</Badge>
		{/if}
	</div>
	{#if description}
		<p class="mt-1 text-muted-foreground text-sm">{description}</p>
	{/if}
</div>
