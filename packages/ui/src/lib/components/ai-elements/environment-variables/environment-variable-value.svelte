<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import {
		useEnvironmentVariable,
		useEnvironmentVariables,
	} from './environment-variables-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		children?: Snippet;
	} = $props();

	const variable = useEnvironmentVariable();
	const ctx = useEnvironmentVariables();

	let displayValue = $derived(
		ctx.showValues ? variable.value : '\u2022'.repeat(Math.min(variable.value.length, 20)),
	);
</script>

<span
	data-slot="environment-variable-value"
	class={cn(
		'font-mono text-muted-foreground text-sm',
		!ctx.showValues && 'select-none',
		className,
	)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		{displayValue}
	{/if}
</span>
