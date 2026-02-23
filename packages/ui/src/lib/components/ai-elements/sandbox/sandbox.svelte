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

<Collapsible
	bind:open
	data-slot="sandbox"
	class={cn('not-prose group mb-4 w-full overflow-hidden rounded-md border', className)}
	{...restProps}
>
	{@render children?.()}
</Collapsible>
