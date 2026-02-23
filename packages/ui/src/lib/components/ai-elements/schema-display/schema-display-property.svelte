<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import type { SchemaProperty } from './schema-display-context.svelte.js';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';
	import {
		Collapsible,
		CollapsibleTrigger,
		CollapsibleContent,
	} from '$lib/components/collapsible/index.js';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import SchemaDisplayProperty from './schema-display-property.svelte';

	let {
		class: className,
		name,
		type,
		required = false,
		description,
		properties,
		items,
		depth = 0,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		name: string;
		type: string;
		required?: boolean;
		description?: string;
		properties?: SchemaProperty[];
		items?: SchemaProperty;
		depth?: number;
	} = $props();

	let hasChildren = $derived(!!properties || !!items);
	let paddingLeft = $derived(40 + depth * 16);
</script>

{#if hasChildren}
	<Collapsible open={depth < 2}>
		<CollapsibleTrigger
			data-slot="schema-display-property"
			class={cn(
				'group flex w-full items-center gap-2 py-3 text-left transition-colors hover:bg-muted/50',
				className,
			)}
			style="padding-left: {paddingLeft}px"
		>
			<ChevronRight
				class="size-4 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-90"
			/>
			<span class="font-mono text-sm">{name}</span>
			<Badge class="text-xs" variant="outline">{type}</Badge>
			{#if required}
				<Badge
					class="bg-red-100 text-red-700 text-xs dark:bg-red-900/30 dark:text-red-400"
					variant="secondary"
				>
					required
				</Badge>
			{/if}
		</CollapsibleTrigger>
		{#if description}
			<p
				class="pb-2 text-muted-foreground text-sm"
				style="padding-left: {paddingLeft + 24}px"
			>
				{description}
			</p>
		{/if}
		<CollapsibleContent>
			<div class="divide-y border-t">
				{#if properties}
					{#each properties as prop (prop.name)}
						<SchemaDisplayProperty
							name={prop.name}
							type={prop.type}
							required={prop.required}
							description={prop.description}
							properties={prop.properties}
							items={prop.items}
							depth={depth + 1}
						/>
					{/each}
				{/if}
				{#if items}
					<SchemaDisplayProperty
						name="{name}[]"
						type={items.type}
						required={items.required}
						description={items.description}
						properties={items.properties}
						items={items.items}
						depth={depth + 1}
					/>
				{/if}
			</div>
		</CollapsibleContent>
	</Collapsible>
{:else}
	<div
		data-slot="schema-display-property"
		class={cn('py-3 pr-4', className)}
		style="padding-left: {paddingLeft}px"
		{...restProps}
	>
		<div class="flex items-center gap-2">
			<span class="size-4"></span>
			<span class="font-mono text-sm">{name}</span>
			<Badge class="text-xs" variant="outline">{type}</Badge>
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
			<p class="mt-1 pl-6 text-muted-foreground text-sm">{description}</p>
		{/if}
	</div>
{/if}
