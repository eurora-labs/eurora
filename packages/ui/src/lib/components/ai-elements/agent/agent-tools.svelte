<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Accordion } from '$lib/components/accordion/index.js';
	import { Accordion as AccordionPrimitive } from 'bits-ui';

	let {
		class: className,
		value = $bindable(),
		type = 'multiple',
		children,
		...restProps
	}: Omit<HTMLAttributes<HTMLDivElement>, 'class'> &
		Omit<AccordionPrimitive.RootProps, 'children'> & {
			class?: string;
			children?: Snippet;
		} = $props();
</script>

<div data-slot="agent-tools" class={cn('space-y-2', className)}>
	<span class="font-medium text-muted-foreground text-sm">Tools</span>
	<Accordion class="rounded-md border" bind:value {type} {...restProps}>
		{@render children?.()}
	</Accordion>
</div>
