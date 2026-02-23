<script lang="ts">
	import * as HoverCard from '$lib/components/hover-card/index.js';

	interface Props {
		openDelay?: number;
		closeDelay?: number;
		children?: import('svelte').Snippet;
	}

	let { openDelay = 0, closeDelay = 0, children, ...restProps }: Props = $props();
</script>

<HoverCard.Root data-slot="attachment-hover-card" {openDelay} {closeDelay} {...restProps}>
	{@render children?.()}
</HoverCard.Root>
