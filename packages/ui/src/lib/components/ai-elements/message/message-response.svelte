<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Streamdown, type StreamdownProps } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import Math from 'svelte-streamdown/math';
	import Mermaid from 'svelte-streamdown/mermaid';
	import StreamingCode from './streaming-code.svelte';

	let {
		class: className,
		components,
		streaming = false,
		...restProps
	}: StreamdownProps & {
		class?: string;
		streaming?: boolean;
	} = $props();

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

<div data-slot="message-response">
	<Streamdown
		class={cn('size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0', className)}
		components={mergedComponents}
		baseTheme="shadcn"
		static={!streaming}
		{...restProps}
	/>
</div>
