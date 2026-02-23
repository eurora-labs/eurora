<script lang="ts">
	import type { Snippet } from 'svelte';
	import { CardContent } from '$lib/components/card/index.js';
	import { CollapsibleContent } from '$lib/components/collapsible/index.js';
	import { Collapsible as CollapsiblePrimitive } from 'bits-ui';

	let {
		children,
		...restProps
	}: CollapsiblePrimitive.ContentProps & {
		children?: Snippet;
	} = $props();
</script>

<CollapsibleContent {...restProps}>
	<CardContent data-slot="plan-content">
		{@render children?.()}
	</CardContent>
</CollapsibleContent>
