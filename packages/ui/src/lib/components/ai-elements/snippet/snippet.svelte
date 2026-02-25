<script lang="ts">
	import type { Snippet as SvelteSnippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { InputGroup } from '$lib/components/input-group/index.js';
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

<InputGroup data-slot="snippet" class={cn('font-mono', className)} {...restProps}>
	{@render children?.()}
</InputGroup>
