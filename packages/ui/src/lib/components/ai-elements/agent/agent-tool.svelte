<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import {
		AccordionItem,
		AccordionTrigger,
		AccordionContent,
	} from '$lib/components/accordion/index.js';
	import { Accordion as AccordionPrimitive } from 'bits-ui';

	let {
		class: className,
		description,
		schema,
		value,
		children,
		...restProps
	}: AccordionPrimitive.ItemProps & {
		description?: string;
		schema?: Record<string, unknown>;
		children?: Snippet;
	} = $props();

	let formatted = $derived(schema ? JSON.stringify(schema, null, 2) : '');
</script>

<AccordionItem
	data-slot="agent-tool"
	class={cn('border-b last:border-b-0', className)}
	{value}
	{...restProps}
>
	<AccordionTrigger class="px-3 py-2 text-sm hover:no-underline">
		{description ?? 'No description'}
	</AccordionTrigger>
	<AccordionContent class="px-3 pb-3">
		{#if children}
			{@render children()}
		{:else}
			<div class="rounded-md bg-muted/50">
				<pre class="overflow-x-auto p-3 text-xs"><code>{formatted}</code></pre>
			</div>
		{/if}
	</AccordionContent>
</AccordionItem>
