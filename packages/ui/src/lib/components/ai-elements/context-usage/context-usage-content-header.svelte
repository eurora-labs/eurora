<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Progress } from '$lib/components/progress/index.js';
	import { getContextUsageContext } from './context-usage-context.svelte.js';

	const PERCENT_MAX = 100;

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getContextUsageContext();

	let usedPercent = $derived(ctx.usedTokens / ctx.maxTokens);
	let displayPct = $derived(
		new Intl.NumberFormat('en-US', {
			maximumFractionDigits: 1,
			style: 'percent',
		}).format(usedPercent),
	);
	let used = $derived(
		new Intl.NumberFormat('en-US', {
			notation: 'compact',
		}).format(ctx.usedTokens),
	);
	let total = $derived(
		new Intl.NumberFormat('en-US', {
			notation: 'compact',
		}).format(ctx.maxTokens),
	);
</script>

<div data-slot="context-usage-content-header" class={cn('w-full space-y-2 p-3', className)} {...rest}>
	{#if children}
		{@render children()}
	{:else}
		<div class="flex items-center justify-between gap-3 text-xs">
			<p>{displayPct}</p>
			<p class="text-muted-foreground font-mono">
				{used} / {total}
			</p>
		</div>
		<div class="space-y-2">
			<Progress class="bg-muted" value={usedPercent * PERCENT_MAX} />
		</div>
	{/if}
</div>
