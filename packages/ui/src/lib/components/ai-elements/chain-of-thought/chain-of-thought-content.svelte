<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Collapsible, CollapsibleContent } from '$lib/components/collapsible/index.js';
	import { getChainOfThoughtContext } from './chain-of-thought-context.svelte.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getChainOfThoughtContext();
</script>

<Collapsible open={ctx.isOpen}>
	<CollapsibleContent
		data-slot="chain-of-thought-content"
		class={cn(
			'mt-2 space-y-3',
			'data-[state=closed]:fade-out-0 data-[state=closed]:slide-out-to-top-2 data-[state=open]:slide-in-from-top-2 text-popover-foreground outline-none data-[state=closed]:animate-out data-[state=open]:animate-in',
			className,
		)}
		{...rest}
	>
		{@render children?.()}
	</CollapsibleContent>
</Collapsible>
