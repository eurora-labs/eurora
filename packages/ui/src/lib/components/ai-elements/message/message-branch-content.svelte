<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getMessageBranchContext } from './message-context.svelte.js';

	let {
		class: className,
		children,
		count = 0,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet<[{ index: number; active: boolean }]>;
		count?: number;
	} = $props();

	const branchState = getMessageBranchContext();

	$effect(() => {
		if (count > 0) {
			branchState.totalBranches = count;
		}
	});
</script>

{#if children}
	{#each Array(count) as _, index}
		<div
			data-slot="message-branch-content"
			class={cn(
				'grid gap-2 overflow-hidden [&>div]:pb-0',
				index === branchState.currentBranch ? 'block' : 'hidden',
				className,
			)}
			{...restProps}
		>
			{@render children({ index, active: index === branchState.currentBranch })}
		</div>
	{/each}
{/if}
