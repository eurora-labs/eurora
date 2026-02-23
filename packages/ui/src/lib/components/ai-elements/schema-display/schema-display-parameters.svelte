<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';
	import {
		Collapsible,
		CollapsibleTrigger,
		CollapsibleContent,
	} from '$lib/components/collapsible/index.js';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { useSchemaDisplay } from './schema-display-context.svelte.js';
	import SchemaDisplayParameter from './schema-display-parameter.svelte';

	let {
		class: className,
		open = $bindable(true),
		children,
		...restProps
	}: {
		class?: string;
		open?: boolean;
		children?: Snippet;
		[key: string]: unknown;
	} = $props();

	const ctx = useSchemaDisplay();
</script>

<Collapsible data-slot="schema-display-parameters" bind:open class={cn(className)} {...restProps}>
	<CollapsibleTrigger
		class="group flex w-full items-center gap-2 px-4 py-3 text-left transition-colors hover:bg-muted/50"
	>
		<ChevronRight
			class="size-4 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-90"
		/>
		<span class="font-medium text-sm">Parameters</span>
		<Badge class="ml-auto text-xs" variant="secondary">
			{ctx.parameters?.length ?? 0}
		</Badge>
	</CollapsibleTrigger>
	<CollapsibleContent>
		<div class="divide-y border-t">
			{#if children}
				{@render children()}
			{:else if ctx.parameters}
				{#each ctx.parameters as param (param.name)}
					<SchemaDisplayParameter
						name={param.name}
						type={param.type}
						required={param.required}
						description={param.description}
						location={param.location}
					/>
				{/each}
			{/if}
		</div>
	</CollapsibleContent>
</Collapsible>
