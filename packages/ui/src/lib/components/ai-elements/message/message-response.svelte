<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Streamdown, type StreamdownProps } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import Math from 'svelte-streamdown/math';
	import Mermaid from 'svelte-streamdown/mermaid';
	import StreamingCode from './streaming-code.svelte';
	import { IncrementalBlocks } from './incremental-blocks.svelte.js';

	let {
		class: className,
		components,
		content = '',
		streaming = false,
		extensions,
		...restProps
	}: StreamdownProps & {
		class?: string;
		streaming?: boolean;
	} = $props();

	// Pre-chunk content into Markdown blocks above the Streamdown layer.
	// Stable blocks are rendered with `static={true}`, which short-circuits
	// Streamdown's internal `parseBlocks` and freezes their token tree.
	// Only the trailing block — during streaming — does live work, and
	// `IncrementalBlocks` re-lexes only the trailing slice rather than the
	// entire content. This is the per-block memoization Jan's React
	// Streamdown gets from `React.memo` on its Block component, but lifted
	// up so we don't have to fork svelte-streamdown.
	const incremental = new IncrementalBlocks();
	const blocks = $derived(incremental.derive(content, extensions ?? []));

	// While streaming, route fenced code blocks through the worker-backed
	// StreamingCode so syntax highlighting runs off the main thread. Once the
	// turn settles, swap back to Streamdown's default Code component and let
	// it render once with `static` mode (no per-paint reparsing).
	const defaultComponents = $derived({
		code: streaming ? StreamingCode : Code,
		math: Math,
		mermaid: Mermaid,
	});
	const mergedComponents = $derived(
		components ? { ...defaultComponents, ...components } : defaultComponents,
	);
</script>

<div data-slot="message-response" class={cn('size-full', className)}>
	{#each blocks as block, i (i)}
		{@const isLast = i === blocks.length - 1}
		<Streamdown
			class={cn(i === 0 && '[&>*:first-child]:mt-0', isLast && '[&>*:last-child]:mb-0')}
			components={mergedComponents}
			baseTheme="shadcn"
			content={block}
			extensions={extensions ?? []}
			static={!streaming || !isLast}
			{...restProps}
		/>
	{/each}
</div>
