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
	data-slot="commit"
	class={cn('rounded-lg border bg-background', className)}
	{...restProps}
>
	{@render children?.()}
</Collapsible>
