<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { TabsList } from '$lib/components/tabs/index.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	} = $props();
</script>

<TabsList
	data-slot="sandbox-tabs-list"
	class={cn('h-auto rounded-none border-0 bg-transparent p-0', className)}
	{...restProps}
>
	{@render children?.()}
</TabsList>
