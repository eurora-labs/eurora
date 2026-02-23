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

	let cacheTokens = $derived(ctx.usage?.cachedInputTokens ?? 0);

	let cacheCost = $derived(
		ctx.modelId
			? computeCost(ctx.modelId, { cacheReads: cacheTokens, input: 0, output: 0 })
			: undefined,
	);

	let cacheCostText = $derived(
		new Intl.NumberFormat('en-US', {
			currency: 'USD',
			style: 'currency',
		}).format(cacheCost ?? 0),
	);
</script>

{#if children}
	{@render children()}
{:else if cacheTokens}
	<div
		data-slot="context-usage-cache"
		class={cn('flex items-center justify-between text-xs', className)}
		{...rest}
	>
		<span class="text-muted-foreground">Cache</span>
		<TokensWithCost tokens={cacheTokens} costText={cacheCostText} />
	</div>
{/if}
