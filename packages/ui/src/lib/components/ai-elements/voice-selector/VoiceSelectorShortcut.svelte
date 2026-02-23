<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import * as Command from '$lib/components/command/index.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { children, ...restProps }: Props = $props();
</script>

<Command.Shortcut data-slot="voice-selector-shortcut" {...restProps}>
	{@render children?.()}
</Command.Shortcut>
