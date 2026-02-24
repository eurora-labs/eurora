<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { useConfirmation } from './confirmation-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();

	const ctx = useConfirmation();
</script>

{#if ctx.state === 'approval-requested'}
	<div
		data-slot="confirmation-actions"
		class={cn('flex items-center justify-end gap-2 self-end', className)}
		{...restProps}
	>
		{@render children?.()}
	</div>
{/if}
