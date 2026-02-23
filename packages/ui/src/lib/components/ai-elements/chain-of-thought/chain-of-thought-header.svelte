<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Collapsible, CollapsibleTrigger } from '$lib/components/collapsible/index.js';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import { getChainOfThoughtContext } from './chain-of-thought-context.svelte.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getChainOfThoughtContext();

	function handleOpenChange(value: boolean) {
		ctx.isOpen = value;
	}
</script>

<Collapsible open={ctx.isOpen} onOpenChange={handleOpenChange}>
	<CollapsibleTrigger
		data-slot="chain-of-thought-header"
		class={cn(
			'text-muted-foreground hover:text-foreground flex w-full items-center gap-2 text-sm transition-colors',
			className,
		)}
		{...rest}
	>
		<BrainIcon class="size-4" />
		<span class="flex-1 text-left">
			{#if children}
				{@render children()}
			{:else}
				Chain of Thought
			{/if}
		</span>
		<ChevronDownIcon
			class={cn('size-4 transition-transform', ctx.isOpen ? 'rotate-180' : 'rotate-0')}
		/>
	</CollapsibleTrigger>
</Collapsible>
