<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Alert } from '$lib/components/alert/index.js';
	import {
		setConfirmation,
		type ToolUIPartApproval,
		type ToolUIPartState,
	} from './confirmation-context.svelte.js';

	let {
		class: className,
		approval = undefined,
		state,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		approval?: ToolUIPartApproval;
		state: ToolUIPartState;
		children?: Snippet;
	} = $props();

	setConfirmation({
		approval: () => approval,
		state: () => state,
	});

	let visible = $derived(
		!!approval && state !== 'input-streaming' && state !== 'input-available',
	);
</script>

{#if visible}
	<Alert data-slot="confirmation" class={cn('flex flex-col gap-2', className)} {...restProps}>
		{@render children?.()}
	</Alert>
{/if}
