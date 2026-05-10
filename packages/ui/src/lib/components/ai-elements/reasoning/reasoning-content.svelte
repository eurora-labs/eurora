<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CollapsibleContent } from '$lib/components/collapsible/index.js';
	import { Streamdown } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import SDMath from 'svelte-streamdown/math';
	import Mermaid from 'svelte-streamdown/mermaid';
	import StreamingCode from '../message/streaming-code.svelte';
	import { IncrementalBlocks } from '../message/incremental-blocks.svelte.js';

	let {
		class: className,
		children,
		streaming = false,
		...restProps
	}: {
		class?: string;
		children: string;
		streaming?: boolean;
		[key: string]: unknown;
	} = $props();

	// Same per-block chunking strategy as `Message.Response` — see comments
	// there. Reasoning streams can be very long for thinking-mode models
	// and benefit even more from incremental parsing.
	const incremental = new IncrementalBlocks();
	const blocks = $derived(incremental.derive(children, []));

	const components = $derived({
		code: streaming ? StreamingCode : Code,
		math: SDMath,
		mermaid: Mermaid,
	});
	const theme = {
		paragraph: { base: 'text-inherit' },
		strong: { base: 'font-semibold text-inherit' },
		h1: { base: 'mt-6 mb-2 text-3xl font-semibold text-inherit' },
		h2: { base: 'mt-6 mb-2 text-2xl font-semibold text-inherit' },
		h3: { base: 'mt-6 mb-2 text-xl font-semibold text-inherit' },
	};
</script>

<CollapsibleContent
	data-slot="reasoning-content"
	class={cn(
		'mt-4 text-sm',
		'data-[state=closed]:fade-out-0 data-[state=closed]:slide-out-to-top-2 data-[state=open]:slide-in-from-top-2 text-muted-foreground outline-none data-[state=closed]:animate-out data-[state=open]:animate-in',
		className,
	)}
	{...restProps}
>
	{#each blocks as block, i (i)}
		{@const isLast = i === blocks.length - 1}
		<Streamdown
			content={block}
			{components}
			{theme}
			baseTheme="shadcn"
			static={!streaming || !isLast}
		/>
	{/each}
</CollapsibleContent>
