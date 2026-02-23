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

<CollapsibleContent>
	<CardContent data-slot="plan-content" {...restProps}>
		{@render children?.()}
	</CardContent>
</CollapsibleContent>
