<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
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

{#if ctx.currentVersion || ctx.newVersion}
	<div
		data-slot="package-info-version"
		class={cn('mt-2 flex items-center gap-2 font-mono text-muted-foreground text-sm', className)}
		{...restProps}
	>
		{#if children}
			{@render children()}
		{:else}
			{#if ctx.currentVersion}
				<span>{ctx.currentVersion}</span>
			{/if}
			{#if ctx.currentVersion && ctx.newVersion}
				<ArrowRight class="size-3" />
			{/if}
			{#if ctx.newVersion}
				<span class="font-medium text-foreground">{ctx.newVersion}</span>
			{/if}
		{/if}
	</div>
{/if}
