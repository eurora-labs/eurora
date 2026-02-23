<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '$lib/components/collapsible/index.js';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { useSchemaDisplay } from './schema-display-context.svelte.js';
	import SchemaDisplayProperty from './schema-display-property.svelte';

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

<Collapsible
	data-slot="schema-display-request"
	bind:open
	class={cn(className)}
	{...restProps}
>
	<CollapsibleTrigger
		class="group flex w-full items-center gap-2 px-4 py-3 text-left transition-colors hover:bg-muted/50"
	>
		<ChevronRight class="size-4 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-90" />
		<span class="font-medium text-sm">Request Body</span>
	</CollapsibleTrigger>
	<CollapsibleContent>
		<div class="border-t">
			{#if children}
				{@render children()}
			{:else if ctx.requestBody}
				{#each ctx.requestBody as prop (prop.name)}
					<SchemaDisplayProperty
						name={prop.name}
						type={prop.type}
						required={prop.required}
						description={prop.description}
						properties={prop.properties}
						items={prop.items}
						depth={0}
					/>
				{/each}
			{/if}
		</div>
	</CollapsibleContent>
</Collapsible>
