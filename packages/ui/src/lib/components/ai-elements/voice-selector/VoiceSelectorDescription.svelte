<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: Props = $props();
</script>

<span
	data-slot="voice-selector-description"
	class={cn('text-muted-foreground text-xs', className)}
	{...restProps}
>
	{@render children?.()}
</span>
