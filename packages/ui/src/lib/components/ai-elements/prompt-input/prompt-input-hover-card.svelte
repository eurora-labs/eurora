<script lang="ts">
	import { HoverCard } from '$lib/components/hover-card/index.js';
	import { LinkPreview as HoverCardPrimitive } from 'bits-ui';

	let { openDelay = 0, closeDelay = 0, ...restProps }: HoverCardPrimitive.RootProps = $props();
</script>

<HoverCard {openDelay} {closeDelay} {...restProps} />
