<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { TabsContent } from '$lib/components/tabs/index.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		value: string;
		children?: Snippet;
		[key: string]: unknown;
	} = $props();
</script>

<TabsContent data-slot="sandbox-tab-content" class={cn('mt-0 text-sm', className)} {...restProps}>
	{@render children?.()}
</TabsContent>
