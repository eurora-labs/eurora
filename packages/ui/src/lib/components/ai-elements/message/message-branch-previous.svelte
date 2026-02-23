<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, type ButtonVariant, type ButtonSize } from '$lib/components/button/index.js';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import { getMessageBranchContext } from './message-context.svelte.js';

	let {
		children,
		size = 'icon-sm' as ButtonSize,
		variant = 'ghost' as ButtonVariant,
		...restProps
	}: {
		children?: Snippet;
		size?: ButtonSize;
		variant?: ButtonVariant;
		[key: string]: any;
	} = $props();

	const branchState = getMessageBranchContext();
</script>

<Button
	data-slot="message-branch-previous"
	aria-label="Previous branch"
	disabled={branchState.totalBranches <= 1}
	onclick={() => branchState.goToPrevious()}
	{size}
	type="button"
	{variant}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<ChevronLeftIcon size={14} />
	{/if}
</Button>
