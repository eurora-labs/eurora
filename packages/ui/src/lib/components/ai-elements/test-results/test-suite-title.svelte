<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getTestSuiteContext } from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTestSuiteContext();
</script>

<span data-slot="test-suite-title" class={cn('font-medium text-sm', className)} {...rest}>
	{#if children}
		{@render children()}
	{:else}
		{ctx.name}
	{/if}
</span>
