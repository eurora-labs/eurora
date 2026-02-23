<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Card } from '$lib/components/card/index.js';
	import { Collapsible } from '$lib/components/collapsible/index.js';
	import { Collapsible as CollapsiblePrimitive } from 'bits-ui';
	import { setPlan } from './plan-context.svelte.js';

	let {
		class: className,
		isStreaming = false,
		open = $bindable(false),
		children,
		...restProps
	}: CollapsiblePrimitive.RootProps & {
		isStreaming?: boolean;
		children?: Snippet;
	} = $props();

	setPlan({ isStreaming: () => isStreaming });
</script>

<Collapsible bind:open data-slot="plan" {...restProps}>
	<Card class={cn('shadow-none', className)}>
		{@render children?.()}
	</Card>
</Collapsible>
