<script lang="ts">
	import * as HoverCard from '$lib/components/hover-card/index.js';
	import type { Snippet } from 'svelte';

	interface Props {
		openDelay?: number;
		closeDelay?: number;
		children?: Snippet;
	}

	let { openDelay = 0, closeDelay = 0, children, ...restProps }: Props = $props();
</script>

<HoverCard.Root {openDelay} {closeDelay} {...restProps}>
	{@render children?.()}
</HoverCard.Root>
