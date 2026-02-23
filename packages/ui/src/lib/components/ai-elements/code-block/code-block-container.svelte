<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	interface Props {
		language: string;
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { language, class: className, children, ...rest }: Props = $props();
</script>

<div
	data-slot="code-block-container"
	data-language={language}
	class={cn(
		'group relative w-full overflow-hidden rounded-md border bg-background text-foreground',
		className,
	)}
	style:content-visibility="auto"
	style:contain-intrinsic-size="auto 200px"
	{...rest}
>
	{@render children?.()}
</div>
