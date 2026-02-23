<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	let {
		children,
		class: className,
		duration = 2,
		spread = 2,
	}: { children: Snippet; class?: string; duration?: number; spread?: number } = $props();

	let shimmerEl = $state<HTMLElement | null>(null);
	let dynamicSpread = $derived((shimmerEl?.textContent?.length ?? 10) * spread);
</script>

<p
	bind:this={shimmerEl}
	data-slot="shimmer"
	class={cn(
		'relative inline-block bg-[length:250%_100%,auto] bg-clip-text text-transparent [background-repeat:no-repeat,padding-box] animate-shimmer',
		className,
	)}
	style:--spread="{dynamicSpread}px"
	style:--shimmer-duration="{duration}s"
	style:background-image="var(--bg), linear-gradient(var(--color-muted-foreground),
	var(--color-muted-foreground))"
>
	{@render children()}
</p>

<style>
	@keyframes shimmer {
		from {
			background-position: 100% center;
		}
		to {
			background-position: 0% center;
		}
	}
	.animate-shimmer {
		--bg: linear-gradient(
			90deg,
			#0000 calc(50% - var(--spread)),
			var(--color-background),
			#0000 calc(50% + var(--spread))
		);
		animation: shimmer var(--shimmer-duration) linear infinite;
	}
</style>
