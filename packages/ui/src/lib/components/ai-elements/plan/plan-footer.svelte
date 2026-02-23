<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { CardFooter } from '$lib/components/card/index.js';

	let {
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();
</script>

<CardFooter data-slot="plan-footer" {...restProps}>
	{@render children?.()}
</CardFooter>
