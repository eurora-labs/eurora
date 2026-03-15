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
	class={cn('mt-2 rounded-md bg-red-50 p-3 dark:bg-red-900/20', className)}
	{...rest}
>
	{@render children?.()}
</div>
