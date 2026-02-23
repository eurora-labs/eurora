<script lang="ts">
	import { HoverCardContent } from '$lib/components/hover-card/index.js';
	import { LinkPreview as HoverCardPrimitive } from 'bits-ui';

	let {
		align = 'start',
		...restProps
	}: HoverCardPrimitive.ContentProps = $props();
</script>

<HoverCardContent data-slot="prompt-input-hover-card-content" {align} {...restProps} />
