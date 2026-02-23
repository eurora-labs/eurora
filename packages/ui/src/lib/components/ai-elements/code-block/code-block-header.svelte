<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<div
	data-slot="code-block-header"
	class={cn(
		'flex items-center justify-between border-b bg-muted/80 px-3 py-2 text-muted-foreground text-xs',
		className,
	)}
	{...rest}
>
	{@render children?.()}
</div>
