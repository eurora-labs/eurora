<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<div data-slot="test-results-title" class={cn('font-medium text-sm', className)} {...rest}>
	{#if children}
		{@render children()}
	{:else}
		Test Results
	{/if}
</div>
