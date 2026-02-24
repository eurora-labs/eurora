<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CardDescription } from '$lib/components/card/index.js';
	import { Shimmer } from '$lib/components/ai-elements/shimmer/index.js';
	import { usePlan } from './plan-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLParagraphElement> & {
		children: string;
	} = $props();

	const plan = usePlan();
</script>

<CardDescription data-slot="plan-description" class={cn('text-balance', className)} {...restProps}>
	{#if plan.isStreaming}
		<Shimmer>{children}</Shimmer>
	{:else}
		{children}
	{/if}
</CardDescription>
