<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		isInternal?: boolean;
		children?: Snippet;
	}

	let { class: className, isInternal = false, children, ...rest }: Props = $props();
</script>

<div
	data-slot="stack-trace-frame"
	class={cn('text-xs', isInternal ? 'text-muted-foreground/50' : 'text-foreground/90', className)}
	{...rest}
>
	{@render children?.()}
</div>
