<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getTestResultsContext, formatDuration } from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTestResultsContext();
</script>

{#if ctx.summary?.duration !== undefined}
	<span
		data-slot="test-results-duration"
		class={cn('text-muted-foreground text-sm', className)}
		{...rest}
	>
		{#if children}
			{@render children()}
		{:else}
			{formatDuration(ctx.summary.duration)}
		{/if}
	</span>
{/if}
