<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';

	let {
		class: className,
		children,
		...restProps
	}: Omit<ButtonProps, 'variant' | 'size'> & {
		children?: Snippet;
	} = $props();
</script>

<Button
	data-slot="queue-item-action"
	class={cn(
		'size-auto rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-muted-foreground/10 hover:text-foreground group-hover:opacity-100',
		className,
	)}
	size="icon"
	type="button"
	variant="ghost"
	{...restProps}
>
	{@render children?.()}
</Button>
