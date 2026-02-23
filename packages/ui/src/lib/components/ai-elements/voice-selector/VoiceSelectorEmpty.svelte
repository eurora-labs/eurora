<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import * as Command from '$lib/components/command/index.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { children, ...restProps }: Props = $props();
</script>

<Command.Empty data-slot="voice-selector-empty" {...restProps}>
	{@render children?.()}
</Command.Empty>
