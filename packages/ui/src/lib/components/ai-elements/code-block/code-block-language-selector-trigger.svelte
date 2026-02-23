<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { SelectTrigger } from '$lib/components/select/index.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<SelectTrigger
	data-slot="code-block-language-selector-trigger"
	class={cn('h-7 border-none bg-transparent px-2 text-xs shadow-none', className)}
	size="sm"
	{...rest}
>
	{@render children?.()}
</SelectTrigger>
