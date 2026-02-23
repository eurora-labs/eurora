<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		children?: Snippet;
	} & Record<string, unknown> = $props();
</script>

<Badge
	data-slot="environment-variable-required"
	class={cn('text-xs', className)}
	variant="secondary"
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		Required
	{/if}
</Badge>
