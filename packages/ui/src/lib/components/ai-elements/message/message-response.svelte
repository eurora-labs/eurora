<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Streamdown, type StreamdownProps, type DeepPartialTheme } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import Math from 'svelte-streamdown/math';
	import Mermaid from 'svelte-streamdown/mermaid';

	let {
		class: className,
		components,
		baseTheme = 'shadcn',
		theme,
		...restProps
	}: StreamdownProps & {
		class?: string;
	} = $props();

	const defaultComponents = { code: Code, math: Math, mermaid: Mermaid };
	const mergedComponents = $derived(components ? { ...defaultComponents, ...components } : defaultComponents);

	const defaultTheme: DeepPartialTheme = {
		h1: { base: 'mt-6 mb-2 text-2xl font-semibold text-foreground' },
		h2: { base: 'mt-5 mb-2 text-xl font-semibold text-foreground' },
		h3: { base: 'mt-4 mb-2 text-lg font-semibold text-foreground' },
		h4: { base: 'mt-3 mb-1 text-base font-semibold text-foreground' },
		h5: { base: 'mt-3 mb-1 text-sm font-semibold text-foreground' },
		h6: { base: 'mt-3 mb-1 text-xs font-semibold text-muted-foreground' },
		link: { base: 'text-primary font-medium underline underline-offset-4 hover:text-primary/80' },
		components: {
			button: 'disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer p-1 text-muted-foreground transition-all hover:text-foreground rounded hover:bg-border flex items-center justify-center size-6 [&>svg]:size-3.5',
		},
	};
	const mergedTheme = $derived(theme ? { ...defaultTheme, ...theme } : defaultTheme);
</script>

<Streamdown
	data-slot="message-response"
	class={cn('size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0', className)}
	components={mergedComponents}
	theme={mergedTheme}
	{baseTheme}
	{...restProps}
/>
