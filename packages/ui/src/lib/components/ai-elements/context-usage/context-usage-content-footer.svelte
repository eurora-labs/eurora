<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { getContextUsageContext } from './context-usage-context.svelte.js';
	import { computeCost } from './cost.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getContextUsageContext();

	let costUSD = $derived(
		ctx.modelId
			? computeCost(ctx.modelId, {
					input: ctx.usage?.inputTokens ?? 0,
					output: ctx.usage?.outputTokens ?? 0,
				})
			: undefined,
	);

	let totalCost = $derived(
		new Intl.NumberFormat('en-US', {
			currency: 'USD',
			style: 'currency',
		}).format(costUSD ?? 0),
	);
</script>

<div
	data-slot="context-usage-content-footer"
	class={cn('bg-secondary flex w-full items-center justify-between gap-3 p-3 text-xs', className)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		<span class="text-muted-foreground">Total cost</span>
		<span>{totalCost}</span>
	{/if}
</div>
