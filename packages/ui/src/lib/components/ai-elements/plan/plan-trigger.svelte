<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import { CollapsibleTrigger } from '$lib/components/collapsible/index.js';
	import ChevronsUpDown from '@lucide/svelte/icons/chevrons-up-down';
	import { Collapsible as CollapsiblePrimitive } from 'bits-ui';

	let {
		class: className,
		children,
		...restProps
	}: CollapsiblePrimitive.TriggerProps & {
		children?: Snippet;
	} = $props();
</script>

<CollapsibleTrigger {...restProps}>
	{#snippet child({ props })}
		<Button
			{...props}
			class={cn('size-8', className)}
			data-slot="plan-trigger"
			size="icon"
			variant="ghost"
		>
			<ChevronsUpDown class="size-4" />
			<span class="sr-only">Toggle plan</span>
		</Button>
	{/snippet}
</CollapsibleTrigger>
