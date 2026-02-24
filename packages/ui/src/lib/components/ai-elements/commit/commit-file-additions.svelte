<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import Plus from '@lucide/svelte/icons/plus';

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
		data-slot="commit-file-additions"
		class={cn('text-green-600 dark:text-green-400', className)}
		{...restProps}
	>
		{#if children}
			{@render children()}
		{:else}
			<Plus class="inline-block size-3" />
			{count}
		{/if}
	</span>
{/if}
