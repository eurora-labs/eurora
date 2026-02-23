<script lang="ts">
	import * as HoverCard from '$lib/components/hover-card/index.js';

	interface Props {
		class?: string;
		children?: import('svelte').Snippet;
	}

	let { children, ...restProps }: Props = $props();
</script>

<HoverCard.Trigger data-slot="attachment-hover-card-trigger" {...restProps}>
	{@render children?.()}
</HoverCard.Trigger>
