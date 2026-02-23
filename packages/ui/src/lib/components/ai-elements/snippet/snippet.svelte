<script lang="ts">
	import type { Snippet as SvelteSnippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { setSnippet } from './snippet-context.svelte.js';

	let {
		class: className,
		code,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		code: string;
		children?: SvelteSnippet;
	} = $props();

	setSnippet({ code: () => code });
</script>

<div
	data-slot="snippet"
	class={cn('flex items-center rounded-md border font-mono text-sm', className)}
	{...restProps}
>
	{@render children?.()}
</div>
