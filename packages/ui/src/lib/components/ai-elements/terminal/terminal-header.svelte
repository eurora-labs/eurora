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
	data-slot="terminal-header"
	class={cn('flex items-center justify-between border-zinc-800 border-b px-4 py-2', className)}
	{...rest}
>
	{@render children?.()}
</div>
