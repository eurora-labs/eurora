<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { ButtonGroup } from '$lib/components/button-group/index.js';
	import { getMessageBranchContext } from './message-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		children?: Snippet;
		[key: string]: any;
	} = $props();

	const branchState = getMessageBranchContext();
</script>

{#if branchState.totalBranches > 1}
	<ButtonGroup
		data-slot="message-branch-selector"
		class={cn(
			"[&>*:not(:first-child)]:rounded-l-md [&>*:not(:last-child)]:rounded-r-md",
			className
		)}
		orientation="horizontal"
		{...restProps}
	>
		{@render children?.()}
	</ButtonGroup>
{/if}
