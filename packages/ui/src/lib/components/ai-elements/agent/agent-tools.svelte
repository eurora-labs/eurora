<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Accordion } from '$lib/components/accordion/index.js';

	interface Props {
		class?: string;
		value?: string[];
		type?: 'multiple';
		children?: Snippet;
	}

	let {
		class: className,
		value = $bindable(),
		type = 'multiple' as const,
		children,
	}: Props = $props();
</script>

<div data-slot="agent-tools" class={cn('space-y-2', className)}>
	<span class="font-medium text-muted-foreground">Tools</span>
	<Accordion class="rounded-md border" bind:value {type}>
		{@render children?.()}
	</Accordion>
</div>
