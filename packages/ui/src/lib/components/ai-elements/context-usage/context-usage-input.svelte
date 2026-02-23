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

	let inputTokens = $derived(ctx.usage?.inputTokens ?? 0);

	let inputCost = $derived(
		ctx.modelId ? computeCost(ctx.modelId, { input: inputTokens, output: 0 }) : undefined,
	);

	let inputCostText = $derived(
		new Intl.NumberFormat('en-US', {
			currency: 'USD',
			style: 'currency',
		}).format(inputCost ?? 0),
	);
</script>

{#if children}
	{@render children()}
{:else if inputTokens}
	<div
		data-slot="context-usage-input"
		class={cn('flex items-center justify-between text-xs', className)}
		{...rest}
	>
		<span class="text-muted-foreground">Input</span>
		<TokensWithCost tokens={inputTokens} costText={inputCostText} />
	</div>
{/if}
