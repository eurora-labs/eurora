<script lang="ts">
	import type { Snippet } from 'svelte';
	import { useConfirmation } from './confirmation-context.svelte.js';

	let { children }: { children?: Snippet } = $props();

	const ctx = useConfirmation();
</script>

{#if ctx.state === 'approval-requested'}
	{@render children?.()}
{/if}
