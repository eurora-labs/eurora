<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Collapsible } from '$lib/components/collapsible/index.js';
	import { Collapsible as CollapsiblePrimitive } from 'bits-ui';

	let {
		class: className,
		open = $bindable(false),
		children,
		...restProps
	}: CollapsiblePrimitive.RootProps & {
		children?: Snippet;
	} = $props();
</script>

<Collapsible
	bind:open
	class={cn('group not-prose mb-4 w-full rounded-md border', className)}
	data-slot="tool"
	{...restProps}
>
	{@render children?.()}
</Collapsible>
