<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<div
	data-slot="stack-trace-frame-source"
	class={cn('mt-1 rounded bg-muted/50 p-2 text-xs', className)}
	{...rest}
>
	{@render children?.()}
</div>
