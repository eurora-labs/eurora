<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Collapsible } from '$lib/components/collapsible/index.js';
	import type { Snippet } from 'svelte';

	let {
		class: className,
		children,
		...restProps
	}: { class?: string; children?: Snippet; [key: string]: any } = $props();
</script>

<Collapsible
	data-slot="sources"
	class={cn('not-prose mb-4 text-primary text-xs', className)}
	{...restProps}
>
	{@render children?.()}
</Collapsible>
