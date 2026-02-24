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
	data-slot="stack-trace-error-type"
	class={cn('shrink-0 font-semibold text-destructive', className)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		{ctx.trace.errorType}
	{/if}
</span>
