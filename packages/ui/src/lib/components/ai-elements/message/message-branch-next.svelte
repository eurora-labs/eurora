<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, type ButtonVariant, type ButtonSize } from '$lib/components/button/index.js';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
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
	data-slot="message-branch-next"
	aria-label="Next branch"
	disabled={branchState.totalBranches <= 1}
	onclick={() => branchState.goToNext()}
	{size}
	type="button"
	{variant}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<ChevronRightIcon size={14} />
	{/if}
</Button>
