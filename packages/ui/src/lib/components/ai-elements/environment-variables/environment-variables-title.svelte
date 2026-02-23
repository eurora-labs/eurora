<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLHeadingElement> & {
		children?: Snippet;
	} = $props();
</script>

<h3
	data-slot="environment-variables-title"
	class={cn('font-medium text-sm', className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		Environment Variables
	{/if}
</h3>
