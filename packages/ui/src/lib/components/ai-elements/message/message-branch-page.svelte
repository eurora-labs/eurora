<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { ButtonGroupText } from '$lib/components/button-group/index.js';
	import { getMessageBranchContext } from './message-context.svelte.js';

	let {
		class: className,
		...restProps
	}: {
		class?: string;
		[key: string]: any;
	} = $props();

	const branchState = getMessageBranchContext();
</script>

<ButtonGroupText
	data-slot="message-branch-page"
	class={cn('border-none bg-transparent text-muted-foreground shadow-none', className)}
	{...restProps}
>
	{branchState.currentBranch + 1} of {branchState.totalBranches}
</ButtonGroupText>
