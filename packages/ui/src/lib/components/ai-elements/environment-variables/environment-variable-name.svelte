<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { useEnvironmentVariable } from './environment-variables-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		children?: Snippet;
	} = $props();

	const variable = useEnvironmentVariable();
</script>

<span
	data-slot="environment-variable-name"
	class={cn('font-mono text-sm', className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		{variable.name}
	{/if}
</span>
