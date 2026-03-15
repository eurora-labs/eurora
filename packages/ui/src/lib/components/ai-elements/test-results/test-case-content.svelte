<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getTestCaseContext, formatDuration } from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTestCaseContext();
</script>

{#if ctx.duration !== undefined}
	<span
		data-slot="test-case-content"
		class={cn('ml-auto text-muted-foreground text-xs', className)}
		{...rest}
	>
		{#if children}
			{@render children()}
		{:else}
			{formatDuration(ctx.duration)}
		{/if}
	</span>
{/if}
