<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Tabs } from '$lib/components/tabs/index.js';
	import { Tabs as TabsPrimitive } from 'bits-ui';

	let {
		class: className,
		children,
		...restProps
	}: TabsPrimitive.RootProps & {
		children?: Snippet;
	} = $props();
</script>

<Tabs data-slot="sandbox-tabs" class={cn('w-full gap-0', className)} {...restProps}>
	{@render children?.()}
</Tabs>
