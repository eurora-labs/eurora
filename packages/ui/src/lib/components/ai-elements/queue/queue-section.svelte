<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Collapsible } from '$lib/components/collapsible/index.js';
	import { Collapsible as CollapsiblePrimitive } from 'bits-ui';

	let {
		class: className,
		open = $bindable(true),
		children,
		...restProps
	}: CollapsiblePrimitive.RootProps & {
		children?: Snippet;
	} = $props();
</script>

<Collapsible bind:open class={cn(className)} data-slot="queue-section" {...restProps}>
	{@render children?.()}
</Collapsible>
