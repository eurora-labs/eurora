<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import Package from '@lucide/svelte/icons/package';
	import { usePackageInfo } from './package-info-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();

	const ctx = usePackageInfo();
</script>

<div data-slot="package-info-name" class={cn('flex items-center gap-2', className)} {...restProps}>
	<Package class="size-4 text-muted-foreground" />
	<span class="font-medium font-mono text-sm">
		{#if children}
			{@render children()}
		{:else}
			{ctx.name}
		{/if}
	</span>
</div>
