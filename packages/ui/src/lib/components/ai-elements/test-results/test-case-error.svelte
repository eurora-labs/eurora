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
	data-slot="test-case-error"
	class={cn('mt-2 rounded-md bg-destructive/10 p-3 text-destructive', className)}
	{...rest}
>
	{@render children?.()}
</div>
