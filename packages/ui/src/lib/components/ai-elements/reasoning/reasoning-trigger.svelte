<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { CollapsibleTrigger } from '$lib/components/collapsible/index.js';
	import { Shimmer } from '$lib/components/ai-elements/shimmer/index.js';
	import { getReasoningContext } from './reasoning-context.svelte.js';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getReasoningContext();
</script>

<CollapsibleTrigger
	data-slot="reasoning-trigger"
	class={cn(
		'text-muted-foreground hover:text-foreground flex w-full items-center gap-2 text-sm transition-colors',
		className,
	)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		<BrainIcon class="size-4" />
		{#if ctx.isStreaming || ctx.duration === 0}
			<Shimmer duration={1}>Thinking...</Shimmer>
		{:else if ctx.duration === undefined}
			<p>Thought for a few seconds</p>
		{:else}
			<p>Thought for {ctx.duration} seconds</p>
		{/if}
		<ChevronDownIcon
			class={cn('size-4 transition-transform', ctx.isOpen ? 'rotate-180' : 'rotate-0')}
		/>
	{/if}
</CollapsibleTrigger>
