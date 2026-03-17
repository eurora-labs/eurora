<script lang="ts">
	import * as HoverCard from '$lib/components/hover-card/index.js';
	import type { Snippet } from 'svelte';

	interface Props {
		class?: string;
		children?: Snippet;
	}

	let { children, ...restProps }: Props = $props();
</script>

<HoverCard.Trigger data-slot="attachment-hover-card-trigger" {...restProps}>
	{@render children?.()}
</HoverCard.Trigger>
