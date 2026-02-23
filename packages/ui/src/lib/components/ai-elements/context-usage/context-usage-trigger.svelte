<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { HoverCardTrigger } from '$lib/components/hover-card/index.js';
	import { Button } from '$lib/components/button/index.js';
	import { getContextUsageContext } from './context-usage-context.svelte.js';
	import ContextUsageIcon from './context-usage-icon.svelte';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getContextUsageContext();

	let usedPercent = $derived(ctx.usedTokens / ctx.maxTokens);
	let renderedPercent = $derived(
		new Intl.NumberFormat('en-US', {
			maximumFractionDigits: 1,
			style: 'percent',
		}).format(usedPercent),
	);
</script>

<HoverCardTrigger data-slot="context-usage-trigger">
	{#if children}
		{@render children()}
	{:else}
		<Button type="button" variant="ghost" class={cn(className)} {...rest}>
			<span class="text-muted-foreground font-medium">
				{renderedPercent}
			</span>
			<ContextUsageIcon />
		</Button>
	{/if}
</HoverCardTrigger>
