<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Shimmer } from '$lib/components/ai-elements/shimmer/index.js';
	import { getTerminalContext } from './terminal-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTerminalContext();
</script>

{#if ctx.isStreaming}
	<div
		data-slot="terminal-status"
		class={cn('flex items-center gap-2 text-xs text-zinc-400', className)}
		{...rest}
	>
		{#if children}
			{@render children()}
		{:else}
			<Shimmer class="w-16">Running...</Shimmer>
		{/if}
	</div>
{/if}
