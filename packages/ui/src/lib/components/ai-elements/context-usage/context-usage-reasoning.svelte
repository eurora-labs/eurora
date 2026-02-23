<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { getContextUsageContext } from './context-usage-context.svelte.js';
	import { computeCost } from './cost.js';
	import TokensWithCost from './tokens-with-cost.svelte';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getContextUsageContext();

	let reasoningTokens = $derived(ctx.usage?.reasoningTokens ?? 0);

	let reasoningCost = $derived(
		ctx.modelId ? computeCost(ctx.modelId, { reasoningTokens }) : undefined,
	);

	let reasoningCostText = $derived(
		new Intl.NumberFormat('en-US', {
			currency: 'USD',
			style: 'currency',
		}).format(reasoningCost ?? 0),
	);
</script>

{#if children}
	{@render children()}
{:else if reasoningTokens}
	<div
		data-slot="context-usage-reasoning"
		class={cn('flex items-center justify-between text-xs', className)}
		{...rest}
	>
		<span class="text-muted-foreground">Reasoning</span>
		<TokensWithCost tokens={reasoningTokens} costText={reasoningCostText} />
	</div>
{/if}
