<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getStackTraceContext } from './stack-trace-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getStackTraceContext();
</script>

<span
	data-slot="stack-trace-error-message"
	class={cn('truncate text-foreground', className)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		{ctx.trace.errorMessage}
	{/if}
</span>
