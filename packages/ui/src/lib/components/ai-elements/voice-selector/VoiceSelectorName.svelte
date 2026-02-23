<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: Props = $props();
</script>

<span data-slot="voice-selector-name" class={cn('flex-1 truncate text-left font-medium', className)} {...restProps}>
	{@render children?.()}
</span>
