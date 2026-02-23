<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import * as Command from '$lib/components/command/index.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { children, ...restProps }: Props = $props();
</script>

<Command.Empty data-slot="mic-selector-empty" {...restProps}>
	{#if children}
		{@render children()}
	{:else}
		No microphone found.
	{/if}
</Command.Empty>
