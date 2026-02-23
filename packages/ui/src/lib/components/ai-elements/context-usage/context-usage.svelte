<script lang="ts">
	import type { Snippet } from 'svelte';
	import { HoverCard } from '$lib/components/hover-card/index.js';
	import {
		ContextUsageState,
		setContextUsageContext,
		type LanguageModelUsage,
		type ModelId,
	} from './context-usage-context.svelte.js';

	interface Props {
		usedTokens: number;
		maxTokens: number;
		usage?: LanguageModelUsage;
		modelId?: ModelId;
		open?: boolean;
		closeDelay?: number;
		openDelay?: number;
		children?: Snippet;
		[key: string]: unknown;
	}

	let {
		usedTokens,
		maxTokens,
		usage,
		modelId,
		open = $bindable(false),
		closeDelay = 0,
		openDelay = 0,
		children,
		...rest
	}: Props = $props();

	let ctx = new ContextUsageState({ usedTokens, maxTokens, usage, modelId });

	setContextUsageContext(ctx);

	$effect(() => {
		ctx.usedTokens = usedTokens;
	});

	$effect(() => {
		ctx.maxTokens = maxTokens;
	});

	$effect(() => {
		ctx.usage = usage;
	});

	$effect(() => {
		ctx.modelId = modelId;
	});
</script>

<HoverCard bind:open {closeDelay} {openDelay} {...rest}>
	{@render children?.()}
</HoverCard>
