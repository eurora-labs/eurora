<script lang="ts">
	import { Streamdown, type StreamdownProps } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import SDMath from 'svelte-streamdown/math';
	import Mermaid from 'svelte-streamdown/mermaid';

	let { ...restProps }: StreamdownProps = $props();

	const components = { code: Code, math: SDMath, mermaid: Mermaid };
	const theme = {
		paragraph: { base: 'text-inherit' },
		strong: { base: 'font-semibold text-inherit' },
		h1: { base: 'mt-6 mb-2 text-3xl font-semibold text-inherit' },
		h2: { base: 'mt-6 mb-2 text-2xl font-semibold text-inherit' },
		h3: { base: 'mt-6 mb-2 text-xl font-semibold text-inherit' },
	};
</script>

<Streamdown {components} {theme} baseTheme="shadcn" {...restProps} />
