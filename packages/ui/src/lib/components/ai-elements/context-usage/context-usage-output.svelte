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

	let outputTokens = $derived(ctx.usage?.outputTokens ?? 0);

	let outputCost = $derived(
		ctx.modelId ? computeCost(ctx.modelId, { input: 0, output: outputTokens }) : undefined,
	);

	let outputCostText = $derived(
		new Intl.NumberFormat('en-US', {
			currency: 'USD',
			style: 'currency',
		}).format(outputCost ?? 0),
	);
</script>

{#if children}
	{@render children()}
{:else if outputTokens}
	<div
		data-slot="context-usage-output"
		class={cn('flex items-center justify-between text-xs', className)}
		{...rest}
	>
		<span class="text-muted-foreground">Output</span>
		<TokensWithCost tokens={outputTokens} costText={outputCostText} />
	</div>
{/if}
