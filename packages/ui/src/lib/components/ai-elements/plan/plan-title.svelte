<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { CardTitle } from '$lib/components/card/index.js';
	import { Shimmer } from '$lib/components/ai-elements/shimmer/index.js';
	import { usePlan } from './plan-context.svelte.js';

	let {
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children: string;
	} = $props();

	const plan = usePlan();
</script>

<CardTitle data-slot="plan-title" {...restProps}>
	{#if plan.isStreaming}
		<Shimmer>{children}</Shimmer>
	{:else}
		{children}
	{/if}
</CardTitle>
