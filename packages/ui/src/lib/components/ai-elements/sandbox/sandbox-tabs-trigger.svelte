<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { TabsTrigger } from '$lib/components/tabs/index.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		value: string;
		children?: Snippet;
		[key: string]: unknown;
	} = $props();
</script>

<TabsTrigger
	data-slot="sandbox-tabs-trigger"
	class={cn(
		'rounded-none border-0 border-transparent border-b-2 px-4 py-2 font-medium text-muted-foreground text-sm transition-colors data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:shadow-none',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</TabsTrigger>
