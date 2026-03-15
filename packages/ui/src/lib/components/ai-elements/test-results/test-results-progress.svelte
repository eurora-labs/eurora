<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getTestResultsContext } from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTestResultsContext();

	let passedPercent = $derived(ctx.summary ? (ctx.summary.passed / ctx.summary.total) * 100 : 0);
	let failedPercent = $derived(ctx.summary ? (ctx.summary.failed / ctx.summary.total) * 100 : 0);
</script>

{#if ctx.summary}
	<div data-slot="test-results-progress" class={cn('space-y-2', className)} {...rest}>
		{#if children}
			{@render children()}
		{:else}
			<div class="flex h-2 overflow-hidden rounded-full bg-muted">
				<div class="bg-green-500 transition-all" style:width="{passedPercent}%"></div>
				<div class="bg-red-500 transition-all" style:width="{failedPercent}%"></div>
			</div>
			<div class="flex justify-between text-muted-foreground text-xs">
				<span>{ctx.summary.passed}/{ctx.summary.total} tests passed</span>
				<span>{passedPercent.toFixed(0)}%</span>
			</div>
		{/if}
	</div>
{/if}
