<script lang="ts">
	import type { Snippet } from 'svelte';
	import { HoverCard } from '$lib/components/hover-card/index.js';

	interface Props {
		open?: boolean;
		closeDelay?: number;
		openDelay?: number;
		children?: Snippet;
		[key: string]: unknown;
	}

	let {
		open = $bindable(false),
		closeDelay = 0,
		openDelay = 0,
		children,
		...rest
	}: Props = $props();
</script>

<HoverCard bind:open {closeDelay} {openDelay} {...rest}>
	{@render children?.()}
</HoverCard>
