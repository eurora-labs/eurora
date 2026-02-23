<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		schema,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		schema: string;
		children?: Snippet;
	} = $props();
</script>

<div data-slot="agent-output" class={cn('space-y-2', className)} {...restProps}>
	<span class="font-medium text-muted-foreground text-sm">Output Schema</span>
	{#if children}
		{@render children()}
	{:else}
		<div class="rounded-md bg-muted/50">
			<pre class="overflow-x-auto p-3 text-xs"><code>{schema}</code></pre>
		</div>
	{/if}
</div>
