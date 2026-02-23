<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as Command from '$lib/components/command/index.js';

	interface Props {
		class?: string;
		children?: Snippet;
	}

	let { children, class: className }: Props = $props();
</script>

<Command.Empty data-slot="mic-selector-empty" class={className}>
	{#if children}
		{@render children()}
	{:else}
		No microphone found.
	{/if}
</Command.Empty>
