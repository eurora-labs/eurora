<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { CardAction } from '$lib/components/card/index.js';

	let {
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();
</script>

<CardAction data-slot="plan-action" {...restProps}>
	{@render children?.()}
</CardAction>
