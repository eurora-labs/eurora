<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		passed?: number;
		failed?: number;
		skipped?: number;
		children?: Snippet;
	}

	let {
		class: className,
		passed = 0,
		failed = 0,
		skipped = 0,
		children,
		...rest
	}: Props = $props();
</script>

<div
	data-slot="test-suite-stats"
	class={cn('ml-auto flex items-center gap-2 text-xs', className)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		{#if passed > 0}
			<span class="text-green-600 dark:text-green-400">{passed} passed</span>
		{/if}
		{#if failed > 0}
			<span class="text-red-600 dark:text-red-400">{failed} failed</span>
		{/if}
		{#if skipped > 0}
			<span class="text-yellow-600 dark:text-yellow-400">{skipped} skipped</span>
		{/if}
	{/if}
</div>
