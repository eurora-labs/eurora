<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as Command from '$lib/components/command/index.js';

	interface Props {
		class?: string;
		heading?: string;
		children?: Snippet;
	}

	let { children, heading, class: className }: Props = $props();
</script>

<Command.Group data-slot="mic-selector-group" {heading} class={className}>
	{@render children?.()}
</Command.Group>
