<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { useSnippet } from './snippet-context.svelte.js';

	let {
		class: className,
		...restProps
	}: HTMLAttributes<HTMLInputElement> & Record<string, unknown> = $props();

	const ctx = useSnippet();
</script>

<input
	data-slot="snippet-input"
	class={cn(
		'flex-1 border-0 bg-transparent px-3 py-2 text-sm text-foreground outline-none',
		className,
	)}
	readonly
	value={ctx.code}
	{...restProps}
/>
