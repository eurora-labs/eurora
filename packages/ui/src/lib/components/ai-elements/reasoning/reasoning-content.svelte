<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CollapsibleContent } from '$lib/components/collapsible/index.js';
	import { Streamdown } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import Math from 'svelte-streamdown/math';
	import Mermaid from 'svelte-streamdown/mermaid';

	let {
		class: className,
		children,
		...restProps
	}: { class?: string; children: string; [key: string]: any } = $props();

	const components = { code: Code, math: Math, mermaid: Mermaid };
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
	<Streamdown content={children} {components} baseTheme="shadcn" />
</CollapsibleContent>
