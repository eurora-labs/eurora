<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		name,
		version,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		name: string;
		version?: string;
		children?: Snippet;
	} = $props();
</script>

<div
	data-slot="package-info-dependency"
	class={cn('flex items-center justify-between', className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<span class="font-mono text-muted-foreground">{name}</span>
		{#if version}
			<span class="font-mono text-xs">{version}</span>
		{/if}
	{/if}
</div>
