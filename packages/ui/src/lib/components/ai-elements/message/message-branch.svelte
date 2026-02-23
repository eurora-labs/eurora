<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { MessageBranchState, setMessageBranchContext } from './message-context.svelte.js';

	let {
		class: className,
		children,
		defaultBranch = 0,
		onBranchChange,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
		defaultBranch?: number;
		onBranchChange?: (branchIndex: number) => void;
	} = $props();

	const branchState = new MessageBranchState(defaultBranch, onBranchChange);
	setMessageBranchContext(branchState);
</script>

<div
	data-slot="message-branch"
	class={cn('grid w-full gap-2 [&>div]:pb-0', className)}
	{...restProps}
>
	{@render children?.()}
</div>
