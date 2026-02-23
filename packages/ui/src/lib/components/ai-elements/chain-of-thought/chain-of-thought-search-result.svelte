<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<Badge
	data-slot="chain-of-thought-search-result"
	class={cn('gap-1 px-2 py-0.5 text-xs font-normal', className)}
	variant="secondary"
	{...rest}
>
	{@render children?.()}
</Badge>
