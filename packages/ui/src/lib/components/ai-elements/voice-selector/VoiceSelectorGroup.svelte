<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import * as Command from '$lib/components/command/index.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		heading?: string;
		children?: Snippet;
	}

	let { children, heading, ...restProps }: Props = $props();
</script>

<Command.Group data-slot="voice-selector-group" {heading} {...restProps}>
	{@render children?.()}
</Command.Group>
