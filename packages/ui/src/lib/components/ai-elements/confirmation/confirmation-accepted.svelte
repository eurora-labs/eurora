<script lang="ts">
	import type { Snippet } from 'svelte';
	import { useConfirmation } from './confirmation-context.svelte.js';

	let { children }: { children?: Snippet } = $props();

	const ctx = useConfirmation();

	let visible = $derived(
		ctx.approval?.approved === true &&
			(ctx.state === 'approval-responded' ||
				ctx.state === 'output-denied' ||
				ctx.state === 'output-available'),
	);
</script>

{#if visible}
	{@render children?.()}
{/if}
