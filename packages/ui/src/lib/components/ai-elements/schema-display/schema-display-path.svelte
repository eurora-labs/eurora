<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { useSchemaDisplay } from './schema-display-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		children?: Snippet;
	} = $props();

	const ctx = useSchemaDisplay();

	let segments = $derived(parsePathSegments(ctx.path));

	function parsePathSegments(path: string): { text: string; isParam: boolean }[] {
		const result: { text: string; isParam: boolean }[] = [];
		const regex = /\{([^}]+)\}|([^{]+)/g;
		let match;
		while ((match = regex.exec(path)) !== null) {
			if (match[1]) {
				result.push({ text: `{${match[1]}}`, isParam: true });
			} else if (match[2]) {
				result.push({ text: match[2], isParam: false });
			}
		}
		return result;
	}
</script>

<span data-slot="schema-display-path" class={cn('font-mono text-sm', className)} {...restProps}>
	{#if children}
		{@render children()}
	{:else}
		{#each segments as segment}
			{#if segment.isParam}
				<span class="text-blue-600 dark:text-blue-400">{segment.text}</span>
			{:else}
				{segment.text}
			{/if}
		{/each}
	{/if}
</span>
