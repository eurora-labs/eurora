<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import Minus from '@lucide/svelte/icons/minus';

	let {
		class: className,
		count,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		count: number;
		children?: Snippet;
	} = $props();
</script>

{#if count > 0}
	<span
		data-slot="commit-file-deletions"
		class={cn('text-red-600 dark:text-red-400', className)}
		{...restProps}
	>
		{#if children}
			{@render children()}
		{:else}
			<Minus class="inline-block size-3" />
			{count}
		{/if}
	</span>
{/if}
