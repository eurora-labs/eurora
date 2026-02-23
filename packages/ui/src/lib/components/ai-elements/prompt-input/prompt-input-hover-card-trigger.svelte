<script lang="ts">
	import { HoverCardTrigger } from '$lib/components/hover-card/index.js';
	import { LinkPreview as HoverCardPrimitive } from 'bits-ui';

	let { ...restProps }: HoverCardPrimitive.TriggerProps = $props();
</script>

<HoverCardTrigger data-slot="prompt-input-hover-card-trigger" {...restProps} />
