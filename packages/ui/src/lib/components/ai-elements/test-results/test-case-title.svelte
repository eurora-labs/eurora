<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getTestCaseContext } from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTestCaseContext();
</script>

<span data-slot="test-case-title" class={cn('flex-1', className)} {...rest}>
	{#if children}
		{@render children()}
	{:else}
		{ctx.name}
	{/if}
</span>
