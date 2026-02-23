<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { CollapsibleTrigger } from '$lib/components/collapsible/index.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		children?: Snippet;
	} & Record<string, unknown> = $props();
</script>

<CollapsibleTrigger
	data-slot="commit-header"
	class={cn(
		'group flex w-full cursor-pointer items-center justify-between gap-4 p-3 text-left transition-colors hover:opacity-80',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</CollapsibleTrigger>
