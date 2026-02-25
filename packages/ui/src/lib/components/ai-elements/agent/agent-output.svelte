<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CodeBlock } from '$lib/components/ai-elements/code-block/index.js';

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
	<span class="font-medium text-muted-foreground">Output Schema</span>
	{#if children}
		{@render children()}
	{:else}
		<div class="rounded-md bg-muted/50">
			<CodeBlock code={schema} language="typescript" />
		</div>
	{/if}
</div>
